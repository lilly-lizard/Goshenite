use super::common::{create_shader_module, CreateDescriptorSetError, CreateShaderError};
use crate::{
    config,
    shaders::shader_interfaces::{CameraPushConstants, SHADER_ENTRY_POINT},
};
use anyhow::Context;
#[allow(unused_imports)]
use log::{debug, error, info, warn};
use std::sync::Arc;
use vulkano::{
    command_buffer::AutoCommandBufferBuilder,
    descriptor_set::{
        allocator::StandardDescriptorSetAllocator, PersistentDescriptorSet, WriteDescriptorSet,
    },
    device::Device,
    image::{view::ImageView, AttachmentImage},
    pipeline::{
        graphics::viewport::{Viewport, ViewportState},
        GraphicsPipeline, Pipeline, PipelineBindPoint,
    },
    render_pass::Subpass,
};

const VERT_SHADER_GLSL_PATH: &str = "assets/shader_binaries/full_screen.vert.spv";
const VERT_SHADER_CIRCLE_PATH: &str = "assets/shader_binaries/full_screen.vert.cxx.spv";
const FRAG_SHADER_PATH: &str = "assets/shader_binaries/scene_lighting.frag.spv";

/// Describes descriptor set indices
mod descriptor {
    /// descriptor set index in `scene_lighting.frag`
    pub const SET_LIGHTING_FRAG: usize = 0;
    /// normal g-buffer input attachment binding in `scene_lighting.frag`
    pub const BINDING_NORMAL: u32 = 0;
    /// primitive-id g-buffer input attachment binding in `scene_lighting.frag`
    pub const BINDING_PRIMITIVE_ID: u32 = 1;
}

/// Defines functionality for reading the g-buffers and calculating the scene color values
pub struct LightingPass {
    pipeline: Arc<GraphicsPipeline>,
    desc_set: Arc<PersistentDescriptorSet>,
}
// Public functions
impl LightingPass {
    pub fn new(
        device: Arc<Device>,
        descriptor_allocator: &StandardDescriptorSetAllocator,
        g_buffer_normal: Arc<ImageView<AttachmentImage>>,
        g_buffer_primitive_id: Arc<ImageView<AttachmentImage>>,
        subpass: Subpass,
    ) -> anyhow::Result<Self> {
        let pipeline = create_pipeline(device.clone(), subpass)?;
        let desc_set = create_desc_set_gbuffers(
            descriptor_allocator,
            pipeline.clone(),
            g_buffer_normal,
            g_buffer_primitive_id,
        )?;
        Ok(Self { pipeline, desc_set })
    }

    /// Updates g-buffer data e.g. when it has been resized
    pub fn update_g_buffers(
        &mut self,
        descriptor_allocator: &StandardDescriptorSetAllocator,
        g_buffer_normal: Arc<ImageView<AttachmentImage>>,
        g_buffer_primitive_id: Arc<ImageView<AttachmentImage>>,
    ) -> anyhow::Result<()> {
        self.desc_set = create_desc_set_gbuffers(
            descriptor_allocator,
            self.pipeline.clone(),
            g_buffer_normal,
            g_buffer_primitive_id,
        )?;
        Ok(())
    }

    /// Records draw commands to a command buffer. Assumes that the command buffer is
    /// already in a render pass state, otherwise an error will be returned.
    pub fn record_commands<L>(
        &self,
        command_buffer: &mut AutoCommandBufferBuilder<L>,
        camera_push_constants: CameraPushConstants,
        viewport: Viewport,
    ) -> anyhow::Result<()> {
        command_buffer
            .set_viewport(0, [viewport])
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                self.desc_set.clone(),
            )
            .push_constants(self.pipeline.layout().clone(), 0, camera_push_constants)
            .draw(3, 1, 0, 0)
            .context("recording lighting pass commands")?;
        Ok(())
    }
}

fn create_pipeline(device: Arc<Device>, subpass: Subpass) -> anyhow::Result<Arc<GraphicsPipeline>> {
    let vert_shader_path = if config::USE_CIRCLE_SHADERS {
        VERT_SHADER_CIRCLE_PATH
    } else {
        VERT_SHADER_GLSL_PATH
    };
    let vert_module = create_shader_module(device.clone(), vert_shader_path)?;
    let vert_shader =
        vert_module
            .entry_point(SHADER_ENTRY_POINT)
            .ok_or(CreateShaderError::MissingEntryPoint(
                vert_shader_path.to_owned(),
            ))?;
    let frag_module = create_shader_module(device.clone(), FRAG_SHADER_PATH)?;
    let frag_shader =
        frag_module
            .entry_point(SHADER_ENTRY_POINT)
            .ok_or(CreateShaderError::MissingEntryPoint(
                FRAG_SHADER_PATH.to_owned(),
            ))?;
    Ok(GraphicsPipeline::start()
        .render_pass(subpass)
        .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
        .vertex_shader(vert_shader, ())
        .fragment_shader(frag_shader, ())
        .build(device.clone())
        .context("creating lighting pass pipeline")?)
}

fn create_desc_set_gbuffers(
    descriptor_allocator: &StandardDescriptorSetAllocator,
    lighting_pipeline: Arc<GraphicsPipeline>,
    g_buffer_normal: Arc<ImageView<AttachmentImage>>,
    g_buffer_primitive_id: Arc<ImageView<AttachmentImage>>,
) -> anyhow::Result<Arc<PersistentDescriptorSet>> {
    PersistentDescriptorSet::new(
        descriptor_allocator,
        lighting_pipeline
            .layout()
            .set_layouts()
            .get(descriptor::SET_LIGHTING_FRAG)
            .ok_or(CreateDescriptorSetError::InvalidDescriptorSetIndex {
                index: descriptor::SET_LIGHTING_FRAG,
                shader_path: FRAG_SHADER_PATH,
            })?
            .to_owned(),
        [
            WriteDescriptorSet::image_view(descriptor::BINDING_NORMAL, g_buffer_normal),
            WriteDescriptorSet::image_view(descriptor::BINDING_PRIMITIVE_ID, g_buffer_primitive_id),
        ],
    )
    .context("creating lighting pass desc set")
}
