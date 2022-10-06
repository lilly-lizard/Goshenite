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
    image::{view::ImageView, StorageImage},
    pipeline::{
        graphics::viewport::{Viewport, ViewportState},
        GraphicsPipeline, Pipeline, PipelineBindPoint,
    },
    render_pass::Subpass,
    sampler::{self, Sampler, SamplerAddressMode, SamplerCreateInfo, SamplerMipmapMode},
};

const VERT_SHADER_PATH: &str = "assets/shader_binaries/blit.vert.spv";
const FRAG_SHADER_PATH: &str = "assets/shader_binaries/blit.frag.spv";

/// Describes descriptor set indices
mod descriptor {
    /// descriptor set index in `post.frag`
    pub const SET_BLIT_FRAG: usize = 0;
    /// render image sampler binding in `post.frag`
    pub const BINDING_SAMPLER: u32 = 0;
}

/// Defines functionality for writing the render image to the swapchain image
pub struct BlitPass {
    pipeline: Arc<GraphicsPipeline>,
    desc_set: Arc<PersistentDescriptorSet>,
    sampler: Arc<Sampler>,
}
// Public functions
impl BlitPass {
    pub fn new(
        device: Arc<Device>,
        descriptor_allocator: &StandardDescriptorSetAllocator,
        render_image: Arc<ImageView<StorageImage>>,
        subpass: Subpass,
    ) -> anyhow::Result<Self> {
        let sampler = create_sampler(device.clone())?;
        let pipeline = create_pipeline(device.clone(), subpass)?;
        let desc_set = create_desc_set(
            descriptor_allocator,
            pipeline.clone(),
            render_image.clone(),
            sampler.clone(),
        )?;
        Ok(Self {
            pipeline,
            desc_set,
            sampler,
        })
    }

    /// Updates render image data e.g. when it has been resized
    pub fn update_render_image(
        &mut self,
        descriptor_allocator: &StandardDescriptorSetAllocator,
        render_image: Arc<ImageView<StorageImage>>,
    ) -> anyhow::Result<()> {
        self.desc_set = create_desc_set(
            descriptor_allocator,
            self.pipeline.clone(),
            render_image,
            self.sampler.clone(),
        )?;
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
            .context("recording blit pass commands")?;
        Ok(())
    }
}

fn create_sampler(device: Arc<Device>) -> anyhow::Result<Arc<Sampler>> {
    sampler::Sampler::new(
        device,
        SamplerCreateInfo {
            mag_filter: sampler::Filter::Linear,
            min_filter: sampler::Filter::Linear,
            address_mode: [SamplerAddressMode::ClampToEdge; 3],
            mipmap_mode: SamplerMipmapMode::Linear,
            ..Default::default()
        },
    )
    .context("creating blit pass sampler")
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
        .context("creating blit pass pipeline")?)
}

fn create_desc_set(
    descriptor_allocator: &StandardDescriptorSetAllocator,
    blit_pipeline: Arc<GraphicsPipeline>,
    render_image: Arc<ImageView<StorageImage>>,
    render_image_sampler: Arc<Sampler>,
) -> anyhow::Result<Arc<PersistentDescriptorSet>> {
    PersistentDescriptorSet::new(
        descriptor_allocator,
        blit_pipeline
            .layout()
            .set_layouts()
            .get(descriptor::SET_BLIT_FRAG)
            .ok_or(CreateDescriptorSetError::InvalidDescriptorSetIndex {
                index: descriptor::SET_BLIT_FRAG,
            })?
            .to_owned(),
        [WriteDescriptorSet::image_view_sampler(
            descriptor::BINDING_SAMPLER,
            render_image,
            render_image_sampler,
        )],
    )
    .context("creating blit pass desc set")
}
