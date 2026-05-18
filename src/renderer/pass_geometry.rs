use crate::renderer::config_renderer::SHADER_ENTRY_POINT;
use crate::{
    engine::object::{object::ObjectId, objects_delta::ObjectsDelta},
    renderer::{
        object_resource_manager::ObjectResourceManager,
        shader_interfaces::vertex_inputs::{BoundingBoxVertex, VulkanVertex},
        vulkan_init::{create_desc_sets_camera, render_pass_indices, write_camera_descriptor_sets},
    },
};
use anyhow::Context;
use ash::vk::{self, SpecializationInfo, SpecializationMapEntry};
use bort_vk::{
    Buffer, ColorBlendState, CommandBuffer, DepthStencilState, DescriptorSet, DescriptorSetLayout,
    DescriptorSetLayoutBinding, DescriptorSetLayoutProperties, Device, DeviceOwned, DynamicState,
    GraphicsPipeline, GraphicsPipelineProperties, MemoryAllocator, PipelineAccess, PipelineLayout,
    PipelineLayoutProperties, Queue, RasterizationState, RenderPass, ShaderModule, ShaderStage,
    ViewportState,
};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::ffi::CString;
use std::sync::Arc;

// descriptor set and binding indices
#[allow(dead_code)]
pub(super) mod descriptor {
    pub const SET_CAMERA: usize = 0;
    pub const BINDING_CAMERA: u32 = 0;

    pub const SET_PRIMITIVE_OPS: usize = 1;
    pub const BINDING_PRIMITIVE_OPS: u32 = 0;
}

/// Render the scene geometry and write to g-buffers
pub struct GeometryPass {
    desc_sets_camera: Vec<DescriptorSet>,

    pipeline: GraphicsPipeline,
    pipeline_selected_object: GraphicsPipeline,
    object_buffer_manager: ObjectResourceManager,
}

// Public functions
impl GeometryPass {
    pub fn new(
        device: Arc<Device>,
        memory_allocator: Arc<MemoryAllocator>,
        render_pass: &RenderPass,
        camera_buffer: &Buffer,
        transfer_queue_family_index: u32,
        render_queue_family_index: u32,
    ) -> anyhow::Result<Self> {
        let desc_sets_camera = create_desc_sets_camera(device.clone(), descriptor::BINDING_CAMERA)?;
        write_camera_descriptor_sets(&desc_sets_camera, camera_buffer, descriptor::BINDING_CAMERA);

        let primitive_ops_desc_set_layout = create_primitive_ops_desc_set_layout(device.clone())?;

        let pipeline_layout = create_pipeline_layout(
            device.clone(),
            desc_sets_camera[0].layout().clone(),
            primitive_ops_desc_set_layout.clone(),
        )?;
        let (pipeline, pipeline_selected_object) = create_pipelines(pipeline_layout, render_pass)?;

        let object_buffer_manager = ObjectResourceManager::new(
            memory_allocator,
            primitive_ops_desc_set_layout,
            transfer_queue_family_index,
            render_queue_family_index,
        )?;

        Ok(Self {
            desc_sets_camera,
            pipeline,
            pipeline_selected_object,
            object_buffer_manager,
        })
    }

    #[inline]
    pub fn update_objects(
        &mut self,
        objects_delta: ObjectsDelta,
        transfer_queue: &Queue,
        render_queue: &Queue,
    ) -> anyhow::Result<()> {
        self.object_buffer_manager
            .update_objects(objects_delta, transfer_queue, render_queue)
    }

    pub fn record_commands(
        &self,
        command_buffer: &CommandBuffer,
        selected_object_id: Option<ObjectId>,
        frame_index: usize,
        viewport: vk::Viewport,
        scissor: vk::Rect2D,
    ) {
        if self.object_buffer_manager.object_count() == 0 {
            trace!("no object buffers found. skipping geometry pass commands...");
            return;
        }

        command_buffer.bind_pipeline(&self.pipeline);
        command_buffer.set_viewport(0, &[viewport]);
        command_buffer.set_scissor(0, &[scissor]);
        command_buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline.pipeline_layout().as_ref(),
            0,
            [&self.desc_sets_camera[frame_index]],
            &[],
        );

        if let Some(selected_object_id) = selected_object_id {
            self.object_buffer_manager.draw_commands_skip_id(
                command_buffer,
                &self.pipeline.pipeline_layout(),
                selected_object_id,
            );

            command_buffer.bind_pipeline(&self.pipeline_selected_object);
            self.object_buffer_manager.draw_commands_object_id(
                command_buffer,
                &self.pipeline_selected_object.pipeline_layout(),
                selected_object_id,
            );
        } else {
            self.object_buffer_manager
                .draw_commands_all(command_buffer, &self.pipeline.pipeline_layout());
        }
    }

    #[inline]
    pub fn object_buffer_manager(&self) -> &ObjectResourceManager {
        &self.object_buffer_manager
    }
}

impl Drop for GeometryPass {
    fn drop(&mut self) {
        trace!("dropping geometry pass...");
    }
}

fn create_primitive_ops_desc_set_layout(
    device: Arc<Device>,
) -> anyhow::Result<Arc<DescriptorSetLayout>> {
    let mut desc_set_layout_props = DescriptorSetLayoutProperties::default();
    desc_set_layout_props.bindings = vec![DescriptorSetLayoutBinding {
        binding: descriptor::BINDING_PRIMITIVE_OPS,
        descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
        descriptor_count: 1,
        stage_flags: vk::ShaderStageFlags::FRAGMENT,
        ..Default::default()
    }];

    let desc_set_layout = DescriptorSetLayout::new(device, desc_set_layout_props)
        .context("creating geometry pass primitive-ops descriptor set layout")?;

    Ok(Arc::new(desc_set_layout))
}

fn create_pipeline_layout(
    device: Arc<Device>,
    desc_set_layout_camera: Arc<DescriptorSetLayout>,
    desc_set_layout_primitive_ops: Arc<DescriptorSetLayout>,
) -> anyhow::Result<Arc<PipelineLayout>> {
    let pipeline_layout_props = PipelineLayoutProperties::new(
        vec![desc_set_layout_camera, desc_set_layout_primitive_ops],
        Vec::new(),
    );

    let pipeline_layout = PipelineLayout::new(device, pipeline_layout_props)
        .context("creating geometry pass pipeline layout")?;

    Ok(Arc::new(pipeline_layout))
}

fn create_pipelines(
    pipeline_layout: Arc<PipelineLayout>,
    render_pass: &RenderPass,
) -> anyhow::Result<(GraphicsPipeline, GraphicsPipeline)> {
    let (vert_stage, frag_stage) = create_shader_stages(pipeline_layout.device().clone())?;

    let dynamic_state =
        DynamicState::new_default(vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);

    let viewport_state = ViewportState::new_dynamic(1, 1);

    let color_blend_state =
        ColorBlendState::new_disabled(render_pass_indices::GBUFFER_COLOR_ATTACHMENT_COUNT);

    let depth_stencil_state = DepthStencilState {
        depth_test_enable: true,
        depth_write_enable: true,
        depth_compare_op: vk::CompareOp::GREATER_OR_EQUAL,
        depth_bounds_test_enable: false,
        stencil_test_enable: false,
        ..Default::default()
    };

    let rasterization_state = RasterizationState {
        // makes sure our fragments are always the far end of the bounding meshes,
        // which allows for a path-tracing miss condition optimization.
        cull_mode: vk::CullModeFlags::FRONT,
        ..Default::default()
    };

    let vertex_input_state = BoundingBoxVertex::vertex_input_state();

    let pipeline_properties = GraphicsPipelineProperties {
        color_blend_state,
        depth_stencil_state,
        dynamic_state,
        rasterization_state,
        subpass_index: render_pass_indices::SUBPASS_GBUFFER as u32,
        vertex_input_state,
        viewport_state,
        ..Default::default()
    };

    let pipeline = GraphicsPipeline::new(
        pipeline_layout.clone(),
        pipeline_properties.clone(),
        &[vert_stage.clone(), frag_stage],
        render_pass,
        None,
    )
    .context("creating geometry pass pipeline")?;

    let spec_constant_entry = [SpecializationMapEntry {
        constant_id: 0,
        offset: 0,
        size: 4,
    }];
    let spec_constant_data: u32 = ash::vk::TRUE;
    let spec_constant_bytes = bytemuck::bytes_of(&spec_constant_data);
    let spec_constant = SpecializationInfo::default()
        .map_entries(&spec_constant_entry)
        .data(spec_constant_bytes);
    let frag_shader_selected_object =
        create_frag_shader_stage_selected_object(pipeline_layout.device().clone(), spec_constant)?;

    let pipeline_selected_object = GraphicsPipeline::new(
        pipeline_layout,
        pipeline_properties,
        &[vert_stage, frag_shader_selected_object],
        render_pass,
        None,
    )
    .context("creating geometry pass pipeline")?;
    Ok((pipeline, pipeline_selected_object))
}

#[cfg(feature = "include-spirv-bytes")]
fn create_shader_stages<'a>(
    device: Arc<Device>,
) -> anyhow::Result<(ShaderStage<'a>, ShaderStage<'a>)> {
    use super::vulkan_init::create_shader_stages_from_bytes;
    let vertex_spv_file = std::io::Cursor::new(
        &include_bytes!("../../assets/shader_binaries/bounding_mesh.vert.spv")[..],
    );
    let frag_spv_file = std::io::Cursor::new(
        &include_bytes!("../../assets/shader_binaries/scene_geometry.frag.spv")[..],
    );
    create_shader_stages_from_bytes(device, vertex_spv_file, frag_spv_file)
        .context("creating geoemetry pass shaders")
}

#[cfg(feature = "include-spirv-bytes")]
fn create_frag_shader_stage_selected_object<'a>(
    device: Arc<Device>,
    spec_constant: SpecializationInfo<'a>,
) -> anyhow::Result<ShaderStage<'a>> {
    let mut frag_spv_file = std::io::Cursor::new(
        &include_bytes!("../../assets/shader_binaries/scene_geometry.frag.spv")[..],
    );
    let frag_shader = Arc::new(ShaderModule::new_from_spirv(device, &mut frag_spv_file)?);
    let shader_stage = ShaderStage::new(
        vk::ShaderStageFlags::FRAGMENT,
        frag_shader,
        CString::new(SHADER_ENTRY_POINT).expect("SHADER_ENTRY_POINT shouldn't contain null byte"),
        Some(spec_constant),
    );
    Ok(shader_stage)
}

#[cfg(not(feature = "include-spirv-bytes"))]
fn create_shader_stages<'a>(
    device: Arc<Device>,
) -> anyhow::Result<(ShaderStage<'a>, ShaderStage<'a>)> {
    use crate::renderer::vulkan_init::create_shader_stages_from_path;
    const VERT_SHADER_PATH: &str = "assets/shader_binaries/bounding_mesh.vert.spv";
    const FRAG_SHADER_PATH: &str = "assets/shader_binaries/scene_geometry.frag.spv";
    create_shader_stages_from_path(device, VERT_SHADER_PATH, FRAG_SHADER_PATH)
        .context("creating geometry pass shaders")
}
