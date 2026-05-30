use crate::renderer::{
    config_renderer::{BOX_INSIDE_STL_PATH, SKYBOX_SIZE},
    shader_interfaces::{
        push_constants::SkyboxPushConstant,
        vertex_inputs::{SkyboxVertex, VulkanVertex},
    },
    vulkan_init::{
        create_desc_sets_camera, create_shader_stage_from_bytes, create_vertex_buffers_from_stl,
        render_pass_indices, write_camera_descriptor_sets,
    },
};
use anyhow::Context;
use ash::vk::{self, ShaderStageFlags};
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

    skybox_vertex_buffer: Buffer,
    skybox_index_buffer: Buffer,
    skybox_index_count: u32,
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
        let (shader_vert, shader_frag_default, shader_frag_object) = create_shader_stages(device)?;
        let pipeline = create_pipeline(
            pipeline_layout,
            render_pass,
            shader_vert,
            shader_frag_default,
        )?;
        let (skybox_vertex_buffer, skybox_index_buffer, skybox_index_count) =
            create_vertex_buffers_from_stl(memory_allocator.clone(), BOX_INSIDE_STL_PATH)?;
        Ok(Self {
            desc_sets_camera,
            pipeline,
            skybox_vertex_buffer,
            skybox_index_buffer,
            skybox_index_count,
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
        command_buffer.bind_vertex_buffers(0, [&self.skybox_vertex_buffer], &[0]);
        command_buffer.bind_index_buffer(&self.skybox_index_buffer, 0, vk::IndexType::UINT32);
        command_buffer.push_constants(
            &self.pipeline.pipeline_layout(),
            vk::ShaderStageFlags::VERTEX,
            0,
            pc_bytes,
        );
        command_buffer.draw_indexed(self.skybox_index_count, 1, 0, 0, 0);
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

fn create_pipeline<'a>(
    pipeline_layout: Arc<PipelineLayout>,
    render_pass: &RenderPass,
    vert_shader: ShaderStage<'a>,
    frag_shader: ShaderStage<'a>,
) -> anyhow::Result<GraphicsPipeline> {
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
        &[vert_shader, frag_shader],
        render_pass,
        None,
    )
    .context("creating overlay pass skybox pipeline")?;

    Ok(pipeline)
}

fn create_shader_stages<'a>(
    device: Arc<Device>,
) -> anyhow::Result<(ShaderStage<'a>, ShaderStage<'a>, ShaderStage<'a>)> {
    let shader_vert = create_shader_stage_from_bytes(
        device.clone(),
        ShaderStageFlags::VERTEX,
        &include_bytes!("../../assets/shader_binaries/skybox.vert.spv")[..],
        None,
    )
    .context("creating skybox shaders")?;
    let shader_frag_default = create_shader_stage_from_bytes(
        device.clone(),
        ShaderStageFlags::FRAGMENT,
        &include_bytes!("../../assets/shader_binaries/skybox_default.frag.spv")[..],
        None,
    )
    .context("creating skybox shaders")?;
    let shader_frag_object = create_shader_stage_from_bytes(
        device.clone(),
        ShaderStageFlags::FRAGMENT,
        &include_bytes!("../../assets/shader_binaries/skybox_object_editor.frag.spv")[..],
        None,
    )
    .context("creating skybox shaders")?;
    Ok((shader_vert, shader_frag_default, shader_frag_object))
}
