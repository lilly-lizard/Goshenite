use super::common::{create_shader_module, CreateDescriptorSetError, CreateShaderError};
use crate::shaders::shader_interfaces::SHADER_ENTRY_POINT;
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

const VERT_SHADER_PATH: &str = "assets/shader_binaries/full_screen.vert.spv";
const FRAG_SHADER_PATH: &str = "assets/shader_binaries/scene_lighting.frag.spv";

/// Describes descriptor set indices
mod descriptor {
    /// descriptor set index in `scene_lighting.frag`
    pub const SET_LIGHTING_FRAG: usize = 0;
    /// render image sampler binding in `scene_lighting.frag`
    pub const BINDING_SAMPLER: u32 = 0;
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
        render_image: Arc<ImageView<AttachmentImage>>,
        subpass: Subpass,
    ) -> anyhow::Result<Self> {
        let pipeline = create_pipeline(device.clone(), subpass)?;
        let desc_set =
            create_desc_set_gbuffer(descriptor_allocator, pipeline.clone(), render_image.clone())?;
        Ok(Self { pipeline, desc_set })
    }

    /// Updates render image data e.g. when it has been resized
    pub fn update_render_image(
        &mut self,
        descriptor_allocator: &StandardDescriptorSetAllocator,
        render_image: Arc<ImageView<AttachmentImage>>,
    ) -> anyhow::Result<()> {
        self.desc_set =
            create_desc_set_gbuffer(descriptor_allocator, self.pipeline.clone(), render_image)?;
        Ok(())
    }

    /// Records draw commands to a command buffer. Assumes that the command buffer is
    /// already in a render pass state, otherwise an error will be returned.
    pub fn record_commands<L>(
        &self,
        command_buffer: &mut AutoCommandBufferBuilder<L>,
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
            .draw(3, 1, 0, 0)
            .context("recording lighting pass commands")?;
        Ok(())
    }
}

fn create_pipeline(device: Arc<Device>, subpass: Subpass) -> anyhow::Result<Arc<GraphicsPipeline>> {
    let vert_module = create_shader_module(device.clone(), VERT_SHADER_PATH)?;
    let vert_shader =
        vert_module
            .entry_point(SHADER_ENTRY_POINT)
            .ok_or(CreateShaderError::MissingEntryPoint(
                VERT_SHADER_PATH.to_string(),
            ))?;
    let frag_module = create_shader_module(device.clone(), FRAG_SHADER_PATH)?;
    let frag_shader =
        frag_module
            .entry_point(SHADER_ENTRY_POINT)
            .ok_or(CreateShaderError::MissingEntryPoint(
                FRAG_SHADER_PATH.to_string(),
            ))?;
    Ok(GraphicsPipeline::start()
        .render_pass(subpass)
        .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
        .vertex_shader(vert_shader, ())
        .fragment_shader(frag_shader, ())
        .build(device.clone())
        .context("creating lighting pass pipeline")?)
}

fn create_desc_set_gbuffer(
    descriptor_allocator: &StandardDescriptorSetAllocator,
    lighting_pipeline: Arc<GraphicsPipeline>,
    render_image: Arc<ImageView<AttachmentImage>>,
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
        [WriteDescriptorSet::image_view(
            descriptor::BINDING_SAMPLER,
            render_image,
        )],
    )
    .context("creating lighting pass desc set")
}
