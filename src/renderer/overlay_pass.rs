use super::{
    config_renderer::GIZMO_ARROW_STL_PATH,
    object_resource_manager::ObjectResourceManager,
    shader_interfaces::vertex_inputs::{BoundingBoxVertex, GizmoVertex, VulkanVertex},
    vulkan_init::{create_camera_descriptor_set_with_binding, render_pass_indices},
};
use crate::{helper::more_errors::IoError, renderer::vulkan_init::write_camera_descriptor_set};
use anyhow::Context;
use ash::vk::{self, BufferUsageFlags};
use bort_vk::{
    AllocationAccess, Buffer, BufferProperties, ColorBlendState, CommandBuffer, DescriptorPool,
    DescriptorPoolProperties, DescriptorSet, DescriptorSetLayout, Device, DeviceOwned,
    DynamicState, GraphicsPipeline, GraphicsPipelineProperties, InputAssemblyState,
    MemoryAllocator, PipelineAccess, PipelineLayout, PipelineLayoutProperties, RasterizationState,
    RenderPass, ShaderStage, ViewportState,
};
use bort_vma::AllocationCreateInfo;
use std::sync::Arc;
use std::{fs::OpenOptions, path::Path};

mod descriptor {
    pub const SET_CAMERA: usize = 0;
    pub const BINDING_CAMERA: u32 = 0;
}

pub struct OverlayPass {
    desc_set_camera: DescriptorSet,
    pipeline_aabb: GraphicsPipeline,
    pipeline_gizmos: GraphicsPipeline,
    arrow_gizmo_vertex_buffer: Buffer,
    arrow_gizmo_index_buffer: Buffer,
}

impl OverlayPass {
    pub fn new(
        memory_allocator: Arc<MemoryAllocator>,
        render_pass: &RenderPass,
        camera_buffer: &Buffer,
    ) -> anyhow::Result<Self> {
        let device = render_pass.device().clone();

        let descriptor_pool = create_descriptor_pool(device.clone())?;
        let desc_set_camera = create_descriptor_set_camera(descriptor_pool)?;
        write_camera_descriptor_set(&desc_set_camera, camera_buffer, descriptor::BINDING_CAMERA);

        let pipeline_layout_aabb =
            create_aabb_pipeline_layout(device.clone(), desc_set_camera.layout().clone())?;
        let pipeline_aabb =
            create_aabb_pipeline(device.clone(), pipeline_layout_aabb.clone(), render_pass)?;

        let pipeline_layout_gizmos =
            create_aabb_pipeline_layout(device.clone(), desc_set_camera.layout().clone())?;
        let pipeline_gizmos =
            create_aabb_pipeline(device.clone(), pipeline_layout_gizmos.clone(), render_pass)?;

        let (arrow_gizmo_vertex_buffer, arrow_gizmo_index_buffer) =
            create_and_upload_gizmo_buffers(memory_allocator)?;

        Ok(Self {
            desc_set_camera,
            pipeline_aabb,
            pipeline_gizmos,
            arrow_gizmo_vertex_buffer,
            arrow_gizmo_index_buffer,
        })
    }

    pub fn record_aabb_overlay_commands(
        &self,
        command_buffer: &CommandBuffer,
        object_resource_manager: &ObjectResourceManager,
        viewport: vk::Viewport,
        scissor: vk::Rect2D,
    ) {
        if object_resource_manager.object_count() == 0 {
            return;
        }

        command_buffer.bind_pipeline(&self.pipeline_aabb);
        command_buffer.set_viewport(0, &[viewport]);
        command_buffer.set_scissor(0, &[scissor]);
        command_buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline_aabb.pipeline_layout().as_ref(),
            0,
            [&self.desc_set_camera],
            &[],
        );

        object_resource_manager.draw_bounding_box_commands(command_buffer);
    }

    pub fn record_gizmo_commands(
        &self,
        command_buffer: &CommandBuffer,
        viewport: vk::Viewport,
        scissor: vk::Rect2D,
    ) {
        command_buffer.bind_pipeline(&self.pipeline_gizmos);
        command_buffer.set_viewport(0, &[viewport]);
        command_buffer.set_scissor(0, &[scissor]);
    }
}

fn create_descriptor_pool(device: Arc<Device>) -> anyhow::Result<Arc<DescriptorPool>> {
    let descriptor_pool_properties = DescriptorPoolProperties {
        max_sets: 1,
        pool_sizes: vec![vk::DescriptorPoolSize {
            ty: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: 1,
        }],
        ..Default::default()
    };

    let descriptor_pool = DescriptorPool::new(device, descriptor_pool_properties)
        .context("creating overlay pass descriptor pool")?;
    Ok(Arc::new(descriptor_pool))
}

fn create_descriptor_set_camera(
    descriptor_pool: Arc<DescriptorPool>,
) -> anyhow::Result<DescriptorSet> {
    create_camera_descriptor_set_with_binding(descriptor_pool, descriptor::BINDING_CAMERA)
        .context("creating geometry pass descriptor set")
}

/// Returns `(vertex_buffer, index_buffer)`
fn create_and_upload_gizmo_buffers(
    memory_allocator: Arc<MemoryAllocator>,
) -> anyhow::Result<(Buffer, Buffer)> {
    let arrow_stl = load_arrow_stl().context("loading gizmo arrow stl")?;

    let vertices = arrow_stl.vertices;
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
        vertices.len() as u64 * 3 * 4, // 3 f32 vertices
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

    let index_buffer_properties = BufferProperties::new_default(
        indices.len() as u64 * 4, // u32 indices
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

    Ok((vertex_buffer, index_buffer))
}

fn load_arrow_stl() -> Result<stl_io::IndexedMesh, IoError> {
    let gizmo_arrow_stl_path = Path::new(GIZMO_ARROW_STL_PATH);
    let mut arrow_stl_file = OpenOptions::new()
        .read(true)
        .open(gizmo_arrow_stl_path)
        .map_err(|e| IoError::read_file_error(e, GIZMO_ARROW_STL_PATH.to_string()))?;
    stl_io::read_stl(&mut arrow_stl_file).map_err(IoError::ReadBufferFailed)
}

fn create_aabb_pipeline_layout(
    device: Arc<Device>,
    desc_set_layout_camera: Arc<DescriptorSetLayout>,
) -> anyhow::Result<Arc<PipelineLayout>> {
    let pipeline_layout_props =
        PipelineLayoutProperties::new(vec![desc_set_layout_camera], Vec::new());

    let pipeline_layout = PipelineLayout::new(device, pipeline_layout_props)
        .context("creating overlay pass aabb pipeline layout")?;

    Ok(Arc::new(pipeline_layout))
}

fn create_aabb_pipeline(
    device: Arc<Device>,
    pipeline_layout: Arc<PipelineLayout>,
    render_pass: &RenderPass,
) -> anyhow::Result<GraphicsPipeline> {
    let (vert_stage, frag_stage) = create_aabb_shader_stages(&device)?;

    let dynamic_state =
        DynamicState::new_default(vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);
    let viewport_state = ViewportState::new_dynamic(1, 1);

    let color_blend_state =
        ColorBlendState::new_default(vec![ColorBlendState::blend_state_disabled()]);

    let rasterization_state = RasterizationState {
        polygon_mode: vk::PolygonMode::LINE,
        line_width: 1_f32,
        ..Default::default()
    };

    let input_assembly_state = InputAssemblyState {
        topology: vk::PrimitiveTopology::TRIANGLE_LIST,
        ..Default::default()
    };

    let vertex_input_state = BoundingBoxVertex::vertex_input_state();

    let pipeline_properties = GraphicsPipelineProperties {
        color_blend_state,
        dynamic_state,
        input_assembly_state,
        rasterization_state,
        subpass_index: render_pass_indices::SUBPASS_DEFERRED as u32,
        vertex_input_state,
        viewport_state,
        ..Default::default()
    };

    let pipeline_aabb = GraphicsPipeline::new(
        pipeline_layout,
        pipeline_properties,
        &[vert_stage, frag_stage],
        render_pass,
        None,
    )
    .context("creating overlay pass aabb pipeline")?;

    Ok(pipeline_aabb)
}

#[cfg(feature = "include-spirv-bytes")]
fn create_aabb_shader_stages(device: &Arc<Device>) -> anyhow::Result<(ShaderStage, ShaderStage)> {
    use super::vulkan_init::create_shader_stages_from_bytes;

    let vertex_spv_file =
        std::io::Cursor::new(&include_bytes!("../../assets/shader_binaries/outlines.vert.spv")[..]);
    let frag_spv_file =
        std::io::Cursor::new(&include_bytes!("../../assets/shader_binaries/outlines.frag.spv")[..]);

    create_shader_stages_from_bytes(device, vertex_spv_file, frag_spv_file)
        .context("creating overlay pass aabb shaders")
}

#[cfg(not(feature = "include-spirv-bytes"))]
fn create_aabb_shader_stages(device: &Arc<Device>) -> anyhow::Result<(ShaderStage, ShaderStage)> {
    use crate::renderer::vulkan_init::create_shader_stages_from_path;

    const VERT_SHADER_PATH: &str = "assets/shader_binaries/outlines.vert.spv";
    const FRAG_SHADER_PATH: &str = "assets/shader_binaries/outlines.frag.spv";

    create_shader_stages_from_path(device, VERT_SHADER_PATH, FRAG_SHADER_PATH)
        .context("creating overlay pass aabb shaders")
}

fn create_gizmos_pipeline_layout(
    device: Arc<Device>,
    desc_set_layout_camera: Arc<DescriptorSetLayout>,
) -> anyhow::Result<Arc<PipelineLayout>> {
    let pipeline_layout_props =
        PipelineLayoutProperties::new(vec![desc_set_layout_camera], Vec::new());

    let pipeline_layout = PipelineLayout::new(device, pipeline_layout_props)
        .context("creating overlay pass gizmos pipeline layout")?;

    Ok(Arc::new(pipeline_layout))
}

fn create_gizmos_pipeline(
    device: Arc<Device>,
    pipeline_layout: Arc<PipelineLayout>,
    render_pass: &RenderPass,
) -> anyhow::Result<GraphicsPipeline> {
    let (vert_stage, frag_stage) = create_gizmos_shader_stages(&device)?;

    let dynamic_state =
        DynamicState::new_default(vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);
    let viewport_state = ViewportState::new_dynamic(1, 1);

    let color_blend_state =
        ColorBlendState::new_default(vec![ColorBlendState::blend_state_disabled()]);

    let rasterization_state = RasterizationState {
        cull_mode: vk::CullModeFlags::BACK,
        ..Default::default()
    };

    let vertex_input_state = GizmoVertex::vertex_input_state();

    let pipeline_properties = GraphicsPipelineProperties {
        color_blend_state,
        dynamic_state,
        rasterization_state,
        subpass_index: render_pass_indices::SUBPASS_DEFERRED as u32,
        vertex_input_state,
        viewport_state,
        ..Default::default()
    };

    let pipeline_gizmos = GraphicsPipeline::new(
        pipeline_layout,
        pipeline_properties,
        &[vert_stage, frag_stage],
        render_pass,
        None,
    )
    .context("creating overlay pass gizmos pipeline")?;

    Ok(pipeline_gizmos)
}

#[cfg(feature = "include-spirv-bytes")]
fn create_gizmos_shader_stages(device: &Arc<Device>) -> anyhow::Result<(ShaderStage, ShaderStage)> {
    use super::vulkan_init::create_shader_stages_from_bytes;

    let vertex_spv_file =
        std::io::Cursor::new(&include_bytes!("../../assets/shader_binaries/gizmos.vert.spv")[..]);
    let frag_spv_file =
        std::io::Cursor::new(&include_bytes!("../../assets/shader_binaries/gizmos.frag.spv")[..]);

    create_shader_stages_from_bytes(device, vertex_spv_file, frag_spv_file)
        .context("creating overlay pass gizmos shaders")
}

#[cfg(not(feature = "include-spirv-bytes"))]
fn create_gizmos_shader_stages(device: &Arc<Device>) -> anyhow::Result<(ShaderStage, ShaderStage)> {
    use crate::renderer::vulkan_init::create_shader_stages_from_path;

    const VERT_SHADER_PATH: &str = "assets/shader_binaries/gizmos.vert.spv";
    const FRAG_SHADER_PATH: &str = "assets/shader_binaries/gizmos.frag.spv";

    create_shader_stages_from_path(device, VERT_SHADER_PATH, FRAG_SHADER_PATH)
        .context("creating overlay pass gizmos shaders")
}
