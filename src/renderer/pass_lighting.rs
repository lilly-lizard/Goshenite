use crate::renderer::vulkan_init::{
    create_desc_sets_camera, create_shader_stage_from_bytes, render_pass_indices,
    write_camera_descriptor_sets,
};
use anyhow::Context;
use ash::vk::{self, ShaderStageFlags};
use bort_vk::{
    Buffer, ColorBlendState, CommandBuffer, DescriptorSet, DescriptorSetLayout,
    DescriptorSetLayoutBinding, DescriptorSetLayoutProperties, Device, DeviceOwned, DynamicState,
    GraphicsPipeline, GraphicsPipelineProperties, Image, ImageView, ImageViewAccess,
    PipelineAccess, PipelineLayout, PipelineLayoutProperties, RenderPass, ShaderStage,
    ViewportState,
};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::sync::Arc;

/// Describes descriptor set indices
#[allow(dead_code)]
mod descriptor {
    pub const SET_G_BUFFERS: usize = 0;
    pub const BINDING_NORMAL: u32 = 0;
    pub const BINDING_ALBEDO: u32 = 1;
    pub const BINDING_PRIMITIVE_ID: u32 = 2;

    pub const SET_CAMERA: usize = 1;
    pub const BINDING_CAMERA: u32 = 0;
}

/// Defines functionality for reading the g-buffers and calculating the scene color values
pub struct LightingPass {
    desc_sets_camera: Vec<DescriptorSet>,
    desc_set_g_buffer: DescriptorSet,

    pipeline: GraphicsPipeline,
}

impl LightingPass {
    pub fn new(
        render_pass: &RenderPass,
        camera_buffer: &Buffer,
        normal_buffer: &ImageView<Image>,
        albedo_buffer: &ImageView<Image>,
        primitive_id_buffer: &ImageView<Image>,
    ) -> anyhow::Result<Self> {
        let device = render_pass.device().clone();
        let desc_set_g_buffer = create_desc_set_gbuffer(device.clone())?;
        let desc_sets_camera = create_desc_sets_camera(device.clone(), descriptor::BINDING_CAMERA)?;
        write_camera_descriptor_sets(&desc_sets_camera, camera_buffer, descriptor::BINDING_CAMERA);
        write_desc_set_gbuffer(
            &desc_set_g_buffer,
            normal_buffer,
            albedo_buffer,
            primitive_id_buffer,
        )?;

        let pipeline_layout = create_pipeline_layout(
            device.clone(),
            desc_sets_camera[0].layout().clone(),
            desc_set_g_buffer.layout().clone(),
        )?;
        let pipeline = create_pipeline(device.clone(), pipeline_layout.clone(), render_pass)?;

        Ok(Self {
            desc_set_g_buffer,
            desc_sets_camera,
            pipeline,
        })
    }

    /// Call whenever the g-buffers change
    pub fn update_g_buffer(
        &mut self,
        normal_buffer: &ImageView<Image>,
        albedo_buffer: &ImageView<Image>,
        primitive_id_buffer: &ImageView<Image>,
    ) -> anyhow::Result<()> {
        write_desc_set_gbuffer(
            &self.desc_set_g_buffer,
            normal_buffer,
            albedo_buffer,
            primitive_id_buffer,
        )
    }

    /// Records draw commands to a command buffer.
    ///
    /// **Assumes that the command buffer is already in a render pass state.**
    pub fn record_commands(
        &self,
        command_buffer: &CommandBuffer,
        frame_index: usize,
        viewport: vk::Viewport,
        scissor: vk::Rect2D,
    ) {
        command_buffer.bind_pipeline(&self.pipeline);
        command_buffer.set_viewport(0, &[viewport]);
        command_buffer.set_scissor(0, &[scissor]);
        command_buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline.pipeline_layout().as_ref(),
            0,
            [&self.desc_set_g_buffer, &self.desc_sets_camera[frame_index]],
            &[],
        );
        command_buffer.draw(3, 1, 0, 0);
    }
}

fn create_desc_set_gbuffer(device: Arc<Device>) -> anyhow::Result<DescriptorSet> {
    let g_buffer_layout_props = DescriptorSetLayoutProperties {
        bindings: vec![
            DescriptorSetLayoutBinding {
                binding: descriptor::BINDING_NORMAL,
                descriptor_type: vk::DescriptorType::INPUT_ATTACHMENT,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
            DescriptorSetLayoutBinding {
                binding: descriptor::BINDING_ALBEDO,
                descriptor_type: vk::DescriptorType::INPUT_ATTACHMENT,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
            DescriptorSetLayoutBinding {
                binding: descriptor::BINDING_PRIMITIVE_ID,
                descriptor_type: vk::DescriptorType::INPUT_ATTACHMENT,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    DescriptorSet::new_from_set_layout(device, g_buffer_layout_props)
        .context("creating geometry pass camera descriptor set")
}

fn write_desc_set_gbuffer(
    desc_set_gbuffer: &DescriptorSet,
    normal_buffer: &impl ImageViewAccess,
    albedo_buffer: &impl ImageViewAccess,
    primitive_id_buffer: &impl ImageViewAccess,
) -> anyhow::Result<()> {
    let normal_buffer_info = vk::DescriptorImageInfo {
        image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        image_view: normal_buffer.handle(),
        ..Default::default()
    };
    let normal_buffer_infos = [normal_buffer_info];

    let albedo_buffer_info = vk::DescriptorImageInfo {
        image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        image_view: albedo_buffer.handle(),
        ..Default::default()
    };
    let albedo_buffer_infos = [albedo_buffer_info];

    let primitive_id_buffer_info = vk::DescriptorImageInfo {
        image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        image_view: primitive_id_buffer.handle(),
        ..Default::default()
    };
    let primitive_id_buffer_infos = [primitive_id_buffer_info];

    let descriptor_write_normal_buffer = vk::WriteDescriptorSet::default()
        .dst_set(desc_set_gbuffer.handle())
        .dst_binding(descriptor::BINDING_NORMAL)
        .descriptor_type(vk::DescriptorType::INPUT_ATTACHMENT)
        .image_info(&normal_buffer_infos);
    let descriptor_write_albedo_buffer = vk::WriteDescriptorSet::default()
        .dst_set(desc_set_gbuffer.handle())
        .dst_binding(descriptor::BINDING_ALBEDO)
        .descriptor_type(vk::DescriptorType::INPUT_ATTACHMENT)
        .image_info(&albedo_buffer_infos);
    let descriptor_write_primitive_id_buffer = vk::WriteDescriptorSet::default()
        .dst_set(desc_set_gbuffer.handle())
        .dst_binding(descriptor::BINDING_PRIMITIVE_ID)
        .descriptor_type(vk::DescriptorType::INPUT_ATTACHMENT)
        .image_info(&primitive_id_buffer_infos);

    desc_set_gbuffer.device().update_descriptor_sets(
        [
            descriptor_write_normal_buffer,
            descriptor_write_albedo_buffer,
            descriptor_write_primitive_id_buffer,
        ],
        [],
    );

    Ok(())
}

fn create_pipeline_layout(
    device: Arc<Device>,
    desc_set_layout_camera: Arc<DescriptorSetLayout>,
    desc_set_layout_g_buffers: Arc<DescriptorSetLayout>,
) -> anyhow::Result<Arc<PipelineLayout>> {
    let pipeline_layout_props = PipelineLayoutProperties::new(
        vec![desc_set_layout_g_buffers, desc_set_layout_camera],
        Vec::new(),
    );

    let pipeline_layout = PipelineLayout::new(device, pipeline_layout_props)
        .context("creating lighting pass pipeline layout")?;

    Ok(Arc::new(pipeline_layout))
}

fn create_pipeline(
    device: Arc<Device>,
    pipeline_layout: Arc<PipelineLayout>,
    render_pass: &RenderPass,
) -> anyhow::Result<GraphicsPipeline> {
    let (vert_stage, frag_stage) = create_shader_stages(device)?;

    let dynamic_state =
        DynamicState::new_default(vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);

    let viewport_state = ViewportState::new_dynamic(1, 1);

    let color_blend_state =
        ColorBlendState::new_disabled(render_pass_indices::DEFERRED_COLOR_ATTACHMENT_COUNT);

    let pipeline_properties = GraphicsPipelineProperties {
        color_blend_state,
        dynamic_state,
        subpass_index: render_pass_indices::SUBPASS_DEFERRED as u32,
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
    .context("creating lighting pass pipeline")?;

    Ok(pipeline)
}

fn create_shader_stages<'a>(
    device: Arc<Device>,
) -> anyhow::Result<(ShaderStage<'a>, ShaderStage<'a>)> {
    let shader_vert = create_shader_stage_from_bytes(
        device.clone(),
        ShaderStageFlags::VERTEX,
        &include_bytes!("../../assets/shader_binaries/full_screen.vert.spv")[..],
        None,
    )
    .context("creating lighting pass shaders")?;
    let shader_frag = create_shader_stage_from_bytes(
        device.clone(),
        ShaderStageFlags::FRAGMENT,
        &include_bytes!("../../assets/shader_binaries/scene_lighting.frag.spv")[..],
        None,
    )
    .context("creating lighting pass shaders")?;
    Ok((shader_vert, shader_frag))
}

impl Drop for LightingPass {
    fn drop(&mut self) {
        trace!("dropping lighting pass...");
    }
}
