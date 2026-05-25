use crate::renderer::{
    config_renderer::{BOX_INSIDE_STL_PATH, SKYBOX_SIZE},
    shader_interfaces::{
        push_constants::SkyboxPushConstant,
        vertex_inputs::{SkyboxVertex, VulkanVertex},
    },
    vulkan_init::{
        create_desc_sets_camera, create_vertex_buffers_from_stl, render_pass_indices,
        write_camera_descriptor_sets,
    },
};
use anyhow::Context;
use ash::vk::{self};
use bort_vk::{
    Buffer, ColorBlendState, CommandBuffer, DepthStencilState, DescriptorSet, DescriptorSetLayout,
    Device, DeviceOwned, DynamicState, GraphicsPipeline, GraphicsPipelineProperties,
    MemoryAllocator, PipelineAccess, PipelineLayout, PipelineLayoutProperties, RasterizationState,
    RenderPass, ShaderStage, ViewportState,
};
use std::sync::Arc;

#[allow(dead_code)]
mod descriptor {
    pub const SET_CAMERA: usize = 0;
    pub const BINDING_CAMERA: u32 = 0;
}

pub struct SkyboxPass {
    desc_sets_camera: Vec<DescriptorSet>,
    pipeline: GraphicsPipeline,

    vertex_buffer: Buffer,
    index_buffer: Buffer,
    index_count: u32,
}

impl SkyboxPass {
    pub fn new(
        memory_allocator: Arc<MemoryAllocator>,
        render_pass: &RenderPass,
        camera_buffer: &Buffer,
    ) -> anyhow::Result<Self> {
        let device = render_pass.device().clone();
        let desc_sets_camera = create_desc_sets_camera(device.clone(), descriptor::BINDING_CAMERA)?;
        write_camera_descriptor_sets(&desc_sets_camera, camera_buffer, descriptor::BINDING_CAMERA);
        let pipeline_layout =
            create_pipeline_layout(device.clone(), desc_sets_camera[0].layout().clone())?;
        let pipeline = create_pipeline(device.clone(), pipeline_layout, render_pass)?;
        let (vertex_buffer, index_buffer, index_count) =
            create_vertex_buffers_from_stl(memory_allocator.clone(), BOX_INSIDE_STL_PATH)?;
        Ok(Self {
            desc_sets_camera,
            pipeline,
            vertex_buffer,
            index_buffer,
            index_count,
        })
    }

    pub fn record_commands(
        &self,
        command_buffer: &CommandBuffer,
        frame_index: usize,
        viewport: vk::Viewport,
        scissor: vk::Rect2D,
    ) {
        let pc_data = SkyboxPushConstant { size: SKYBOX_SIZE };
        let pc_bytes = bytemuck::bytes_of(&pc_data);

        command_buffer.bind_pipeline(&self.pipeline);
        command_buffer.set_viewport(0, &[viewport]);
        command_buffer.set_scissor(0, &[scissor]);
        command_buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::GRAPHICS,
            &self.pipeline.pipeline_layout(),
            0,
            [&self.desc_sets_camera[frame_index]],
            &[],
        );
        command_buffer.bind_vertex_buffers(0, [&self.vertex_buffer], &[0]);
        command_buffer.bind_index_buffer(&self.index_buffer, 0, vk::IndexType::UINT32);
        command_buffer.push_constants(
            &self.pipeline.pipeline_layout(),
            vk::ShaderStageFlags::VERTEX,
            0,
            pc_bytes,
        );
        command_buffer.draw_indexed(self.index_count, 1, 0, 0, 0);
    }
}

fn create_pipeline_layout(
    device: Arc<Device>,
    desc_set_layout_camera: Arc<DescriptorSetLayout>,
) -> anyhow::Result<Arc<PipelineLayout>> {
    let push_constant_range = vk::PushConstantRange::default()
        .stage_flags(vk::ShaderStageFlags::VERTEX)
        .offset(0)
        .size(std::mem::size_of::<SkyboxPushConstant>() as u32);

    let pipeline_layout_props =
        PipelineLayoutProperties::new(vec![desc_set_layout_camera], vec![push_constant_range]);

    let pipeline_layout = PipelineLayout::new(device, pipeline_layout_props)
        .context("creating skybox pass pipeline layout")?;

    Ok(Arc::new(pipeline_layout))
}

fn create_pipeline(
    device: Arc<Device>,
    pipeline_layout: Arc<PipelineLayout>,
    render_pass: &RenderPass,
) -> anyhow::Result<GraphicsPipeline> {
    let (vert_stage, frag_stage) = create_shader_stages(device)?;

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

    let dynamic_state =
        DynamicState::new_default(vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);

    let rasterization_state = RasterizationState {
        cull_mode: vk::CullModeFlags::BACK,
        ..Default::default()
    };

    let vertex_input_state = SkyboxVertex::vertex_input_state();

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
    .context("creating overlay pass skybox pipeline")?;

    Ok(pipeline)
}

#[cfg(feature = "include-spirv-bytes")]
fn create_shader_stages<'a>(
    device: Arc<Device>,
) -> anyhow::Result<(ShaderStage<'a>, ShaderStage<'a>)> {
    use super::vulkan_init::create_shader_stages_from_bytes;
    let vertex_spv_file =
        std::io::Cursor::new(&include_bytes!("../../assets/shader_binaries/skybox.vert.spv")[..]);
    let frag_spv_file =
        std::io::Cursor::new(&include_bytes!("../../assets/shader_binaries/skybox.frag.spv")[..]);
    create_shader_stages_from_bytes(device, vertex_spv_file, frag_spv_file)
        .context("creating overlay pass skybox shaders")
}

#[cfg(not(feature = "include-spirv-bytes"))]
fn create_shader_stages<'a>(
    device: Arc<Device>,
) -> anyhow::Result<(ShaderStage<'a>, ShaderStage<'a>)> {
    use crate::renderer::vulkan_init::create_shader_stages_from_path;
    const VERT_SHADER_PATH: &str = "assets/shader_binaries/skybox.vert.spv";
    const FRAG_SHADER_PATH: &str = "assets/shader_binaries/skybox.frag.spv";
    create_shader_stages_from_path(device, VERT_SHADER_PATH, FRAG_SHADER_PATH)
        .context("creating overlay pass skybox shaders")
}
