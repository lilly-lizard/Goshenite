use super::common::{
    create_shader_module, CreateDescriptorSetError, CreatePipelineError, CreateShaderError,
};
use crate::{helper::from_err_impl::from_err_impl, shaders::shader_interfaces::SHADER_ENTRY_POINT};
use std::fmt;
use std::sync::Arc;
use vulkano::{
    command_buffer::{AutoCommandBufferBuilder, PipelineExecutionError},
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::Device,
    format::Format,
    image::{view::ImageView, StorageImage},
    pipeline::{
        graphics::{
            render_pass::PipelineRenderingCreateInfo,
            viewport::{Viewport, ViewportState},
        },
        GraphicsPipeline, Pipeline, PipelineBindPoint,
    },
    sampler::{
        self, Sampler, SamplerAddressMode, SamplerCreateInfo, SamplerCreationError,
        SamplerMipmapMode,
    },
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
impl BlitPass {
    pub fn new(
        device: Arc<Device>,
        swapchain_image_format: Format,
        render_image: Arc<ImageView<StorageImage>>,
    ) -> Result<Self, BlitPassError> {
        let sampler = Self::create_sampler(device.clone())?;
        let pipeline = Self::create_pipeline(device.clone(), swapchain_image_format)?;
        let desc_set =
            Self::create_desc_set(pipeline.clone(), render_image.clone(), sampler.clone())?;
        Ok(Self {
            pipeline,
            desc_set,
            sampler,
        })
    }

    /// Updates render image data e.g. when it has been resized
    pub fn update_render_image(
        &mut self,
        render_image: Arc<ImageView<StorageImage>>,
    ) -> Result<(), BlitPassError> {
        self.desc_set =
            Self::create_desc_set(self.pipeline.clone(), render_image, self.sampler.clone())?;
        Ok(())
    }

    /// Records draw commands to a command buffer. Assumes that the command buffer is
    /// already in a render pass state, otherwise an error will be returned.
    pub fn record_commands<L>(
        &self,
        command_buffer: &mut AutoCommandBufferBuilder<L>,
        viewport: Viewport,
    ) -> Result<(), PipelineExecutionError> {
        command_buffer
            .set_viewport(0, [viewport])
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                self.desc_set.clone(),
            )
            .draw(3, 1, 0, 0)?;
        Ok(())
    }
}
// Private functions
impl BlitPass {
    fn create_sampler(device: Arc<Device>) -> Result<Arc<Sampler>, SamplerCreationError> {
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
    }

    fn create_pipeline(
        device: Arc<Device>,
        swapchain_image_format: Format,
    ) -> Result<Arc<GraphicsPipeline>, CreatePipelineError> {
        let vert_module = create_shader_module(device.clone(), VERT_SHADER_PATH)?;
        let vert_shader = vert_module.entry_point(SHADER_ENTRY_POINT).ok_or(
            CreateShaderError::MissingEntryPoint(VERT_SHADER_PATH.to_string()),
        )?;
        let frag_module = create_shader_module(device.clone(), FRAG_SHADER_PATH)?;
        let frag_shader = frag_module.entry_point(SHADER_ENTRY_POINT).ok_or(
            CreateShaderError::MissingEntryPoint(FRAG_SHADER_PATH.to_string()),
        )?;
        Ok(GraphicsPipeline::start()
            .render_pass(PipelineRenderingCreateInfo {
                color_attachment_formats: vec![Some(swapchain_image_format)],
                ..Default::default()
            })
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .vertex_shader(vert_shader, ())
            .fragment_shader(frag_shader, ())
            .build(device.clone())?)
    }

    fn create_desc_set(
        blit_pipeline: Arc<GraphicsPipeline>,
        render_image: Arc<ImageView<StorageImage>>,
        render_image_sampler: Arc<Sampler>,
    ) -> Result<Arc<PersistentDescriptorSet>, CreateDescriptorSetError> {
        Ok(PersistentDescriptorSet::new(
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
        )?)
    }
}

// ~~~ Errors ~~~

/// Errors encountered when creating a new `BlitPass`
#[derive(Debug)]
pub enum BlitPassError {
    /// Failed to create render image sampler
    SamplerCreationError(SamplerCreationError),
    /// Errors encountered when creating a pipeline
    CreatePipelineError(CreatePipelineError),
    /// Errors encountered when creating a descriptor set
    CreateDescriptorSetError(CreateDescriptorSetError),
}
impl std::error::Error for BlitPassError {}
impl fmt::Display for BlitPassError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BlitPassError::SamplerCreationError(e) => e.fmt(f),
            BlitPassError::CreatePipelineError(e) => e.fmt(f),
            BlitPassError::CreateDescriptorSetError(e) => e.fmt(f),
        }
    }
}
from_err_impl!(BlitPassError, SamplerCreationError);
from_err_impl!(BlitPassError, CreatePipelineError);
from_err_impl!(BlitPassError, CreateDescriptorSetError);
