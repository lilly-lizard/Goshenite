use super::{
    object_resource_manager::ObjectResourceManager,
    shader_interfaces::vertex_inputs::{BoundingBoxVertex, VulkanVertex},
    vulkan_init::render_pass_indices,
};
use crate::renderer::vulkan_init::{
    create_desc_sets_camera, create_shader_stage_from_bytes, write_camera_descriptor_sets,
};
use anyhow::Context;
use ash::vk::{self, ShaderStageFlags};
use bort_vk::{
    Buffer, ColorBlendState, CommandBuffer, DescriptorSet, DescriptorSetLayout, Device,
    DeviceOwned, DynamicState, GraphicsPipeline, GraphicsPipelineProperties, InputAssemblyState,
    PipelineAccess, PipelineLayout, PipelineLayoutProperties, RasterizationState, RenderPass,
    ShaderStage, ViewportState,
};
use std::sync::Arc;

#[allow(dead_code)]
mod descriptor {
    pub const SET_CAMERA: usize = 0;
    pub const BINDING_CAMERA: u32 = 0;
}

pub struct OverlayPass {
    desc_sets_camera: Vec<DescriptorSet>,
    pipeline_aabb: GraphicsPipeline,
}

impl OverlayPass {
    pub fn new(render_pass: &RenderPass, camera_buffer: &Buffer) -> anyhow::Result<Self> {
        let device = render_pass.device().clone();

        let desc_sets_camera = create_desc_sets_camera(device.clone(), descriptor::BINDING_CAMERA)?;
        write_camera_descriptor_sets(&desc_sets_camera, camera_buffer, descriptor::BINDING_CAMERA);

        let pipeline_layout_aabb =
            create_aabb_pipeline_layout(device.clone(), desc_sets_camera[0].layout().clone())?;
        let pipeline_aabb =
            create_aabb_pipeline(device.clone(), pipeline_layout_aabb.clone(), render_pass)?;

        Ok(Self {
            desc_sets_camera,
            pipeline_aabb,
        })
    }

    pub fn record_aabb_overlay_commands(
        &self,
        command_buffer: &CommandBuffer,
        frame_index: usize,
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
            [&self.desc_sets_camera[frame_index]],
            &[],
        );

        object_resource_manager.draw_bounding_box_commands(command_buffer);
    }
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
    let (vert_stage, frag_stage) = create_aabb_shader_stages(device)?;

    let dynamic_state =
        DynamicState::new_default(vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);
    let viewport_state = ViewportState::new_dynamic(1, 1);

    let color_blend_state =
        ColorBlendState::new_disabled(render_pass_indices::DEFERRED_COLOR_ATTACHMENT_COUNT);

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

fn create_aabb_shader_stages<'a>(
    device: Arc<Device>,
) -> anyhow::Result<(ShaderStage<'a>, ShaderStage<'a>)> {
    let shader_vert = create_shader_stage_from_bytes(
        device.clone(),
        ShaderStageFlags::VERTEX,
        &include_bytes!("../../assets/shader_binaries/outlines.vert.spv")[..],
        None,
    )
    .context("creating AABB overlay shaders")?;
    let shader_frag = create_shader_stage_from_bytes(
        device.clone(),
        ShaderStageFlags::FRAGMENT,
        &include_bytes!("../../assets/shader_binaries/outlines.frag.spv")[..],
        None,
    )
    .context("creating AABB overlay shaders")?;
    Ok((shader_vert, shader_frag))
}
