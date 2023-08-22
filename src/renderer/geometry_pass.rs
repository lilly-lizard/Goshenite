use super::{
    config_renderer::SHADER_ENTRY_POINT,
    object_resource_manager::ObjectResourceManager,
    shader_interfaces::{uniform_buffers::CameraUniformBuffer, vertex_inputs::BoundingBoxVertex},
    vulkan_init::render_pass_indices,
};
use crate::engine::object::{object_collection::ObjectCollection, objects_delta::ObjectsDelta};
use anyhow::Context;
use ash::vk;
use bort_vk::{
    Buffer, ColorBlendState, CommandBuffer, DepthStencilState, DescriptorPool,
    DescriptorPoolProperties, DescriptorSet, DescriptorSetLayout, DescriptorSetLayoutBinding,
    DescriptorSetLayoutProperties, Device, DeviceOwned, DynamicState, GraphicsPipeline,
    GraphicsPipelineProperties, MemoryAllocator, PipelineAccess, PipelineLayout,
    PipelineLayoutProperties, Queue, RasterizationState, RenderPass, ShaderModule, ShaderStage,
    ViewportState,
};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::{ffi::CString, mem, sync::Arc};

// descriptor set and binding indices
pub(super) mod descriptor {
    pub const SET_CAMERA: usize = 0;
    pub const BINDING_CAMERA: u32 = 0;

    pub const SET_PRIMITIVE_OPS: usize = 1;
    pub const BINDING_PRIMITIVE_OPS: u32 = 0;
}

/// Render the scene geometry and write to g-buffers
pub struct GeometryPass {
    device: Arc<Device>,

    desc_set_camera: Arc<DescriptorSet>,

    pipeline: Arc<GraphicsPipeline>,
    object_buffer_manager: ObjectResourceManager,
}

// Public functions
impl GeometryPass {
    pub fn new(
        device: Arc<Device>,
        memory_allocator: Arc<MemoryAllocator>,
        render_pass: &RenderPass,
        camera_buffer: &Buffer,
        queue_family_index: u32,
    ) -> anyhow::Result<Self> {
        let descriptor_pool = create_descriptor_pool(device.clone())?;

        let desc_set_camera = create_desc_set_camera(descriptor_pool.clone())?;
        write_desc_set_camera(&desc_set_camera, camera_buffer)?;

        let primitive_ops_desc_set_layout = create_primitive_ops_desc_set_layout(device.clone())?;

        let pipeline_layout = create_pipeline_layout(
            device.clone(),
            desc_set_camera.layout().clone(),
            primitive_ops_desc_set_layout.clone(),
        )?;

        let pipeline = create_pipeline(pipeline_layout, render_pass)?;

        let object_buffer_manager = ObjectResourceManager::new(
            memory_allocator,
            primitive_ops_desc_set_layout,
            queue_family_index,
        )?;

        Ok(Self {
            device,
            desc_set_camera,
            pipeline,
            object_buffer_manager,
        })
    }

    /// Good for initializing
    pub fn upload_overwrite_object_collection(
        &mut self,
        object_collection: &ObjectCollection,
        queue: &Queue,
    ) -> anyhow::Result<()> {
        self.object_buffer_manager.reset_staging_buffer_offsets();

        let objects = object_collection.objects();

        // added objects
        for (object_id, object_ref) in objects {
            trace!("uploading object id = {:?} to gpu buffer", *object_id);
            let object = &*object_ref.as_ref().borrow();
            self.object_buffer_manager.update_or_push(object, queue)?;
        }

        Ok(())
    }

    pub fn update_objects(
        &mut self,
        object_collection: &ObjectCollection,
        objects_delta: ObjectsDelta,
        queue: &Queue,
    ) -> anyhow::Result<()> {
        self.object_buffer_manager.reset_staging_buffer_offsets();

        // freed objects
        for free_id in objects_delta.remove {
            if let Some(_removed_index) = self.object_buffer_manager.remove(free_id) {
                trace!("removing object buffer id = {:?}", free_id);
            } else {
                debug!(
                    "object buffer id = {:?} was requested to be removed but not found!",
                    free_id
                );
            }
        }

        // added objects
        for set_id in objects_delta.update {
            if let Some(object_ref) = object_collection.get(set_id) {
                trace!("adding or updating object buffer id = {:?}", set_id);
                let object = &*object_ref.as_ref().borrow();
                self.object_buffer_manager.update_or_push(object, queue)?;
            } else {
                warn!(
                    "requsted update for object id = {:?} but wasn't found in object collection!",
                    set_id
                );
            }
        }

        Ok(())
    }

    pub fn update_camera_descriptor_set(&self, camera_buffer: &Buffer) -> anyhow::Result<()> {
        write_desc_set_camera(&self.desc_set_camera, camera_buffer)
    }

    pub fn record_commands(
        &self,
        command_buffer: &CommandBuffer,
        viewport: vk::Viewport,
        scissor: vk::Rect2D,
    ) -> anyhow::Result<()> {
        if self.object_buffer_manager.object_count() == 0 {
            trace!("no object buffers found. skipping geometry pass commands...");
            return Ok(());
        }

        let device_ash = self.device.inner();
        let command_buffer_handle = command_buffer.handle();
        let descriptor_set_handles = [self.desc_set_camera.handle()];

        unsafe {
            device_ash.cmd_bind_pipeline(
                command_buffer_handle,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline.handle(),
            );
            device_ash.cmd_set_viewport(command_buffer_handle, 0, &[viewport]);
            device_ash.cmd_set_scissor(command_buffer_handle, 0, &[scissor]);
            device_ash.cmd_bind_descriptor_sets(
                command_buffer_handle,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline.pipeline_layout().handle(),
                0,
                &descriptor_set_handles,
                &[],
            );
        }

        self.object_buffer_manager
            .draw_commands(command_buffer, &self.pipeline)?;

        Ok(())
    }
}

impl Drop for GeometryPass {
    fn drop(&mut self) {
        trace!("dropping geometry pass...");
    }
}

fn create_descriptor_pool(device: Arc<Device>) -> anyhow::Result<Arc<DescriptorPool>> {
    let descriptor_pool_props = DescriptorPoolProperties {
        max_sets: 1,
        pool_sizes: vec![vk::DescriptorPoolSize {
            ty: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: 1,
        }],
        ..Default::default()
    };

    let descriptor_pool = DescriptorPool::new(device, descriptor_pool_props)
        .context("creating geometry pass descriptor pool")?;

    Ok(Arc::new(descriptor_pool))
}

fn create_desc_set_camera(
    descriptor_pool: Arc<DescriptorPool>,
) -> anyhow::Result<Arc<DescriptorSet>> {
    let desc_set_layout_props =
        DescriptorSetLayoutProperties::new_default(vec![DescriptorSetLayoutBinding {
            binding: descriptor::BINDING_CAMERA,
            descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: 1,
            stage_flags: vk::ShaderStageFlags::FRAGMENT | vk::ShaderStageFlags::VERTEX,
            ..Default::default()
        }]);

    let desc_set_layout = Arc::new(
        DescriptorSetLayout::new(descriptor_pool.device().clone(), desc_set_layout_props)
            .context("creating geometry pass camera descriptor set layout")?,
    );

    let desc_set = descriptor_pool
        .allocate_descriptor_set(desc_set_layout)
        .context("allocating geometry pass camera descriptor set")?;

    Ok(Arc::new(desc_set))
}

fn write_desc_set_camera(
    desc_set_camera: &DescriptorSet,
    camera_buffer: &Buffer,
) -> anyhow::Result<()> {
    let camera_buffer_info = vk::DescriptorBufferInfo {
        buffer: camera_buffer.handle(),
        offset: 0,
        range: mem::size_of::<CameraUniformBuffer>() as vk::DeviceSize,
    };

    let descriptor_writes = [vk::WriteDescriptorSet::builder()
        .dst_set(desc_set_camera.handle())
        .dst_binding(descriptor::BINDING_CAMERA)
        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
        .buffer_info(&[camera_buffer_info])
        .build()];

    unsafe {
        desc_set_camera
            .device()
            .inner()
            .update_descriptor_sets(&descriptor_writes, &[]);
    }

    Ok(())
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

fn create_pipeline(
    pipeline_layout: Arc<PipelineLayout>,
    render_pass: &RenderPass,
) -> anyhow::Result<Arc<GraphicsPipeline>> {
    let (vert_stage, frag_stage) = create_shader_stages(pipeline_layout.device())?;

    let dynamic_state =
        DynamicState::new_default(vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);

    let viewport_state = ViewportState::new_dynamic(1, 1);

    let color_blend_state =
        ColorBlendState::new_default(vec![ColorBlendState::blend_state_disabled(); 2]);

    let depth_stencil_state = DepthStencilState {
        depth_test_enable: true,
        depth_write_enable: true,
        depth_compare_op: vk::CompareOp::GREATER_OR_EQUAL,
        depth_bounds_test_enable: false,
        stencil_test_enable: false,
        ..Default::default()
    };

    let raster_state = RasterizationState {
        // makes sure our fragments are always the far end of the bounding meshes,
        // which allows for a path-tracing miss condition optimization.
        cull_mode: vk::CullModeFlags::FRONT,
        ..Default::default()
    };

    let mut pipeline_properties = GraphicsPipelineProperties::default();
    pipeline_properties.subpass_index = render_pass_indices::SUBPASS_GBUFFER as u32;
    pipeline_properties.dynamic_state = dynamic_state;
    pipeline_properties.color_blend_state = color_blend_state;
    pipeline_properties.vertex_input_state = BoundingBoxVertex::vertex_input_state();
    pipeline_properties.viewport_state = viewport_state;
    pipeline_properties.depth_stencil_state = depth_stencil_state;
    pipeline_properties.rasterization_state = raster_state;

    let pipeline = GraphicsPipeline::new(
        pipeline_layout,
        pipeline_properties,
        &[vert_stage, frag_stage],
        render_pass,
        None,
    )
    .context("creating geometry pass pipeline")?;

    Ok(Arc::new(pipeline))
}

#[cfg(feature = "include-spirv-bytes")]
fn create_shader_stages(device: &Arc<Device>) -> anyhow::Result<(ShaderStage, ShaderStage)> {
    let mut vertex_spv_file = std::io::Cursor::new(
        &include_bytes!("../../assets/shader_binaries/bounding_mesh.vert.spv")[..],
    );
    let vert_shader = Arc::new(
        ShaderModule::new_from_spirv(device.clone(), &mut vertex_spv_file)
            .context("creating lighting pass vertex shader")?,
    );
    let vert_stage = ShaderStage::new(
        vk::ShaderStageFlags::VERTEX,
        vert_shader,
        CString::new(SHADER_ENTRY_POINT).context("converting shader entry point to c-string")?,
        None,
    );

    let mut frag_spv_file = std::io::Cursor::new(
        &include_bytes!("../../assets/shader_binaries/scene_geometry.frag.spv")[..],
    );
    let frag_shader = Arc::new(
        ShaderModule::new_from_spirv(device.clone(), &mut frag_spv_file)
            .context("creating lighting pass fragment shader")?,
    );
    let frag_stage = ShaderStage::new(
        vk::ShaderStageFlags::FRAGMENT,
        frag_shader,
        CString::new(SHADER_ENTRY_POINT).context("converting shader entry point to c-string")?,
        None,
    );

    Ok((vert_stage, frag_stage))
}

#[cfg(not(feature = "include-spirv-bytes"))]
fn create_shader_stages(device: &Arc<Device>) -> anyhow::Result<(ShaderStage, ShaderStage)> {
    const VERT_SHADER_PATH: &str = "assets/shader_binaries/bounding_mesh.vert.spv";
    const FRAG_SHADER_PATH: &str = "assets/shader_binaries/scene_geometry.frag.spv";

    let vert_shader = Arc::new(
        ShaderModule::new_from_file(device.clone(), VERT_SHADER_PATH)
            .context("creating lighting pass vertex shader")?,
    );
    let vert_stage = ShaderStage::new(
        vk::ShaderStageFlags::VERTEX,
        vert_shader,
        CString::new(SHADER_ENTRY_POINT).context("converting shader entry point to c-string")?,
        None,
    );

    let frag_shader = Arc::new(
        ShaderModule::new_from_file(device.clone(), FRAG_SHADER_PATH)
            .context("creating lighting pass fragment shader")?,
    );
    let frag_stage = ShaderStage::new(
        vk::ShaderStageFlags::FRAGMENT,
        frag_shader,
        CString::new(SHADER_ENTRY_POINT).context("converting shader entry point to c-string")?,
        None,
    );

    Ok((vert_stage, frag_stage))
}
