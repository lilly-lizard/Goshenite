#![allow(dead_code)]
use super::{
    config_renderer::GIZMO_ARROW_STL_PATH,
    shader_interfaces::{
        uniform_buffers::CameraUniformBuffer,
        vertex_inputs::{GizmoVertex, VulkanVertex},
    },
    vulkan_init::render_pass_indices,
};
use crate::{
    helper::more_errors::IoError,
    renderer::{
        shader_interfaces::{
            primitive_op_buffer::{
                PRIMITIVE_ID_GIZMO_X, PRIMITIVE_ID_GIZMO_Y, PRIMITIVE_ID_GIZMO_Z,
            },
            push_constants::GizmosPushConstant,
            uniform_buffers::GizmoUniformBuffer,
        },
        vulkan_init::camera_ubo_descriptor_set_layout,
    },
};
use anyhow::Context;
use ash::vk::{self, BufferUsageFlags};
use bort_vk::{
    AllocationAccess, Buffer, BufferProperties, ColorBlendState, CommandBuffer, DepthStencilState,
    DescriptorSet, DescriptorSetLayout, DescriptorSetLayoutBinding, DescriptorSetLayoutProperties,
    Device, DeviceOwned, DynamicState, GraphicsPipeline, GraphicsPipelineProperties,
    MemoryAllocator, PipelineAccess, PipelineLayout, PipelineLayoutProperties, RasterizationState,
    RenderPass, ShaderStage, ViewportState,
};
use bort_vma::AllocationCreateInfo;
use glam::{Mat4, Vec4};
use std::{fs::OpenOptions, mem::size_of, path::Path};
use std::{mem, sync::Arc};

#[allow(dead_code)]
mod descriptor {
    pub const SET_CAMERA: usize = 0;
    pub const BINDING_CAMERA: u32 = 0;
    pub const SET_GIZMO: usize = 1;
    pub const BINDING_GIZMO: u32 = 0;
}

pub struct GizmoPass {
    desc_set_camera: DescriptorSet,
    desc_set_gizmo_params: DescriptorSet,
    pipeline: GraphicsPipeline,

    arrow_vertex_buffer: Buffer,
    arrow_index_buffer: Buffer,
    arrow_index_count: u32,
    arrow_instance_buffer: Buffer,
}

impl GizmoPass {
    pub fn new(
        memory_allocator: Arc<MemoryAllocator>,
        render_pass: &RenderPass,
        camera_buffer: &Buffer,
        gizmo_params_buffer: &Buffer,
    ) -> anyhow::Result<Self> {
        let device = render_pass.device().clone();

        let (desc_set_camera, desc_set_gizmo_params) = create_descriptor_sets(device.clone())?;
        write_descriptor_set(
            &desc_set_camera,
            camera_buffer,
            mem::size_of::<CameraUniformBuffer>() as vk::DeviceSize,
            descriptor::BINDING_CAMERA,
        );
        write_descriptor_set(
            &desc_set_gizmo_params,
            gizmo_params_buffer,
            mem::size_of::<GizmoUniformBuffer>() as vk::DeviceSize,
            descriptor::BINDING_GIZMO,
        );

        let pipeline_layout = create_pipeline_layout(
            device.clone(),
            desc_set_camera.layout().clone(),
            desc_set_gizmo_params.layout().clone(),
        )?;
        let pipeline = create_pipeline(device.clone(), pipeline_layout, render_pass)?;

        let (arrow_vertex_buffer, arrow_index_buffer, arrow_index_count) =
            create_and_upload_gizmo_buffers(memory_allocator.clone())?;
        let arrow_instance_buffer = create_and_upload_instance_buffer(memory_allocator)?;

        Ok(Self {
            desc_set_camera,
            desc_set_gizmo_params,
            pipeline,
            arrow_vertex_buffer,
            arrow_index_buffer,
            arrow_index_count,
            arrow_instance_buffer,
        })
    }

    pub fn record_commands(
        &self,
        command_buffer: &CommandBuffer,
        viewport: vk::Viewport,
        scissor: vk::Rect2D,
    ) {
        let color_red: [f32; 3] = [0.8, 0.1, 0.1];
        let color_green: [f32; 3] = [0.1, 0.8, 0.1];
        let color_blue: [f32; 3] = [0.1, 0.1, 0.8];
        let data_x = GizmosPushConstant {
            color: color_red,
            object_id: PRIMITIVE_ID_GIZMO_X,
        };
        let data_y = GizmosPushConstant {
            color: color_green,
            object_id: PRIMITIVE_ID_GIZMO_Y,
        };
        let data_z = GizmosPushConstant {
            color: color_blue,
            object_id: PRIMITIVE_ID_GIZMO_Z,
        };
        let pc_bytes_x = bytemuck::bytes_of(&data_x);
        let pc_bytes_y = bytemuck::bytes_of(&data_y);
        let pc_bytes_z = bytemuck::bytes_of(&data_z);

        let layout = self.pipeline.pipeline_layout().as_ref();
        command_buffer.bind_pipeline(&self.pipeline);
        command_buffer.set_viewport(0, &[viewport]);
        command_buffer.set_scissor(0, &[scissor]);
        command_buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::GRAPHICS,
            layout,
            0,
            [&self.desc_set_camera, &self.desc_set_gizmo_params],
            &[],
        );
        command_buffer.bind_vertex_buffers(
            0,
            [&self.arrow_vertex_buffer, &self.arrow_instance_buffer],
            &[0, 0],
        );
        command_buffer.bind_index_buffer(&self.arrow_index_buffer, 0, vk::IndexType::UINT32);

        // x arrow
        command_buffer.push_constants(layout, vk::ShaderStageFlags::FRAGMENT, 0, pc_bytes_x);
        command_buffer.draw_indexed(self.arrow_index_count, 1, 0, 0, 0);

        // y arrow
        command_buffer.push_constants(layout, vk::ShaderStageFlags::FRAGMENT, 0, pc_bytes_y);
        command_buffer.draw_indexed(self.arrow_index_count, 1, 0, 0, 1);

        // z arrow
        command_buffer.push_constants(layout, vk::ShaderStageFlags::FRAGMENT, 0, pc_bytes_z);
        command_buffer.draw_indexed(self.arrow_index_count, 1, 0, 0, 2);
    }
}

fn create_descriptor_sets(device: Arc<Device>) -> anyhow::Result<(DescriptorSet, DescriptorSet)> {
    let camera_layout_properties = camera_ubo_descriptor_set_layout(descriptor::BINDING_CAMERA);
    let gizmo_layout_properties =
        DescriptorSetLayoutProperties::new_default(vec![DescriptorSetLayoutBinding {
            binding: descriptor::BINDING_GIZMO,
            descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: 1,
            stage_flags: vk::ShaderStageFlags::FRAGMENT | vk::ShaderStageFlags::VERTEX,
            ..Default::default()
        }]);
    let layout_properties = vec![camera_layout_properties, gizmo_layout_properties];

    let mut sets = DescriptorSet::new_from_set_layouts(device, layout_properties)
        .context("creating gizmo descriptor sets")?;

    let gizmo_set = sets.pop().expect("should have created 2 descrpitor sets");
    let camera_set = sets.pop().expect("should have created 2 descrpitor sets");
    Ok((camera_set, gizmo_set))
}

pub fn write_descriptor_set(
    desc_set: &DescriptorSet,
    buffer: &Buffer,
    size: vk::DeviceSize,
    binding: u32,
) {
    let buffer_info = vk::DescriptorBufferInfo {
        buffer: buffer.handle(),
        offset: 0,
        range: size,
    };
    let buffer_infos = [buffer_info];

    let descriptor_write = vk::WriteDescriptorSet::default()
        .dst_set(desc_set.handle())
        .dst_binding(binding)
        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
        .buffer_info(&buffer_infos);

    desc_set
        .device()
        .update_descriptor_sets([descriptor_write], []);
}

/// Returns `(vertex_buffer, index_buffer, index_count)`
fn create_and_upload_gizmo_buffers(
    memory_allocator: Arc<MemoryAllocator>,
) -> anyhow::Result<(Buffer, Buffer, u32)> {
    let arrow_stl = load_arrow_stl().context("loading gizmo arrow stl")?;

    let vertices: Vec<[f32; 4]> = arrow_stl
        .vertices
        .iter()
        .map(|vertex| [vertex[0], vertex[1], vertex[2], 1.0])
        .collect();

    let indices: Vec<u32> = arrow_stl
        .faces
        .iter()
        .flat_map(|indexed_triangle| {
            [
                // these are called vertices but are actually indices
                indexed_triangle.vertices[0] as u32,
                indexed_triangle.vertices[1] as u32,
                indexed_triangle.vertices[2] as u32,
            ]
        })
        .collect();

    let buffer_allocation_info = AllocationCreateInfo {
        required_flags: vk::MemoryPropertyFlags::HOST_VISIBLE, // todo staging buffer
        preferred_flags: vk::MemoryPropertyFlags::DEVICE_LOCAL,
        ..AllocationCreateInfo::default()
    };

    let vertex_buffer_properties = BufferProperties::new_default(
        vertices.len() as u64 * size_of::<[f32; 4]>() as u64,
        BufferUsageFlags::VERTEX_BUFFER | BufferUsageFlags::TRANSFER_DST,
    );
    let mut vertex_buffer = Buffer::new(
        memory_allocator.clone(),
        vertex_buffer_properties,
        buffer_allocation_info.clone(),
    )
    .context("creating gizmo vertex buffer")?;

    vertex_buffer
        .write_iter(vertices, 0)
        .context("uploading gizmo vertices")?;

    let index_count = indices.len();
    let index_buffer_properties = BufferProperties::new_default(
        index_count as u64 * 4, // u32 indices
        BufferUsageFlags::INDEX_BUFFER | BufferUsageFlags::TRANSFER_DST,
    );
    let mut index_buffer = Buffer::new(
        memory_allocator,
        index_buffer_properties,
        buffer_allocation_info,
    )
    .context("creating gizmo vertex buffer")?;

    index_buffer
        .write_iter(indices, 0)
        .context("uploading gizmo indices")?;

    Ok((vertex_buffer, index_buffer, index_count as u32))
}

fn create_and_upload_instance_buffer(
    memory_allocator: Arc<MemoryAllocator>,
) -> anyhow::Result<Buffer> {
    // note that the arrow model points up (z axis)
    #[rustfmt::skip]
    let x_rot = Mat4::from_cols(
        // rotate 90 around y axis
        Vec4::new(0., 0.,-1., 0.),
        Vec4::new(0., 1., 0., 0.),
        Vec4::new(1., 0., 0., 0.),
        Vec4::new(0., 0., 0., 1.),
    );
    #[rustfmt::skip]
    let y_rot = Mat4::from_cols(
        // rotate 90 around x axis
        Vec4::new(1., 0., 0., 0.),
        Vec4::new(0., 0.,-1., 0.),
        Vec4::new(0., 1., 0., 0.),
        Vec4::new(0., 0., 0., 1.),
    );
    let z_rot = Mat4::IDENTITY;
    let matrices = [x_rot, y_rot, z_rot];

    let instance_buffer_properties = BufferProperties::new_default(
        3 * size_of::<Mat4>() as u64,
        BufferUsageFlags::VERTEX_BUFFER | BufferUsageFlags::TRANSFER_DST,
    );

    let buffer_allocation_info = AllocationCreateInfo {
        required_flags: vk::MemoryPropertyFlags::HOST_VISIBLE, // todo staging buffer
        preferred_flags: vk::MemoryPropertyFlags::DEVICE_LOCAL,
        ..AllocationCreateInfo::default()
    };

    let mut instance_buffer = Buffer::new(
        memory_allocator.clone(),
        instance_buffer_properties,
        buffer_allocation_info,
    )
    .context("creating gizmo instance buffer")?;

    instance_buffer
        .write_iter(matrices, 0)
        .context("uploading gizmo instance vertex data")?;

    Ok(instance_buffer)
}

fn load_arrow_stl() -> Result<stl_io::IndexedMesh, IoError> {
    let gizmo_arrow_stl_path = Path::new(GIZMO_ARROW_STL_PATH);
    let mut arrow_stl_file = OpenOptions::new()
        .read(true)
        .open(gizmo_arrow_stl_path)
        .map_err(|e| IoError::read_file_error(e, GIZMO_ARROW_STL_PATH.to_string()))?;
    stl_io::read_stl(&mut arrow_stl_file).map_err(IoError::ReadBufferFailed)
}

fn create_pipeline_layout(
    device: Arc<Device>,
    desc_set_layout_camera: Arc<DescriptorSetLayout>,
    desc_set_layout_gizmo: Arc<DescriptorSetLayout>,
) -> anyhow::Result<Arc<PipelineLayout>> {
    let push_constant_range = vk::PushConstantRange::default()
        .stage_flags(vk::ShaderStageFlags::FRAGMENT)
        .offset(0)
        .size(std::mem::size_of::<GizmosPushConstant>() as u32);

    let pipeline_layout_props = PipelineLayoutProperties::new(
        vec![desc_set_layout_camera, desc_set_layout_gizmo],
        vec![push_constant_range],
    );

    let pipeline_layout = PipelineLayout::new(device, pipeline_layout_props)
        .context("creating overlay pass gizmos pipeline layout")?;

    Ok(Arc::new(pipeline_layout))
}

fn create_pipeline(
    device: Arc<Device>,
    pipeline_layout: Arc<PipelineLayout>,
    render_pass: &RenderPass,
) -> anyhow::Result<GraphicsPipeline> {
    let (vert_stage, frag_stage) = create_shader_stages(device)?;

    let color_blend_state = ColorBlendState::new_default(vec![
        ColorBlendState::blend_state_disabled(
        );
        render_pass_indices::GBUFFER_COLOR_ATTACHMENT_COUNT
    ]);

    let depth_stencil_state = DepthStencilState {
        depth_test_enable: false,
        depth_write_enable: true,
        ..Default::default()
    };

    let dynamic_state =
        DynamicState::new_default(vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);

    let rasterization_state = RasterizationState {
        cull_mode: vk::CullModeFlags::BACK,
        ..Default::default()
    };

    let vertex_input_state = GizmoVertex::vertex_input_state();

    let viewport_state = ViewportState::new_dynamic(1, 1);

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
        pipeline_layout,
        pipeline_properties,
        &[vert_stage, frag_stage],
        render_pass,
        None,
    )
    .context("creating overlay pass gizmos pipeline")?;

    Ok(pipeline)
}

#[cfg(feature = "include-spirv-bytes")]
fn create_shader_stages<'a>(
    device: Arc<Device>,
) -> anyhow::Result<(ShaderStage<'a>, ShaderStage<'a>)> {
    use super::vulkan_init::create_shader_stages_from_bytes;

    let vertex_spv_file =
        std::io::Cursor::new(&include_bytes!("../../assets/shader_binaries/gizmos.vert.spv")[..]);
    let frag_spv_file =
        std::io::Cursor::new(&include_bytes!("../../assets/shader_binaries/gizmos.frag.spv")[..]);

    create_shader_stages_from_bytes(device, vertex_spv_file, frag_spv_file)
        .context("creating overlay pass gizmos shaders")
}

#[cfg(not(feature = "include-spirv-bytes"))]
fn create_shader_stages<'a>(
    device: Arc<Device>,
) -> anyhow::Result<(ShaderStage<'a>, ShaderStage<'a>)> {
    use crate::renderer::vulkan_init::create_shader_stages_from_path;

    const VERT_SHADER_PATH: &str = "assets/shader_binaries/gizmos.vert.spv";
    const FRAG_SHADER_PATH: &str = "assets/shader_binaries/gizmos.frag.spv";

    create_shader_stages_from_path(device, VERT_SHADER_PATH, FRAG_SHADER_PATH)
        .context("creating overlay pass gizmos shaders")
}
