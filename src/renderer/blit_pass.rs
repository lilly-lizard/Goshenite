use super::render_manager::{create_shader_module, RenderManagerError, RenderManagerUnrecoverable};
use std::sync::Arc;
use vulkano::{
    command_buffer::{AutoCommandBufferBuilder, DrawError, PrimaryAutoCommandBuffer},
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
    sampler::Sampler,
};

/// Describes descriptor set indices
pub mod descriptor {
    /// descriptor set index in `post.frag`
    pub const SET_BLIT_FRAG: usize = 0;
    /// render image sampler binding in `post.frag`
    pub const BINDING_SAMPLER: u32 = 0;
}

/// Defines functionality for writing the render image to the swapchain image
pub struct BlitPass {
    pipeline: Arc<GraphicsPipeline>,
    desc_set: Arc<PersistentDescriptorSet>,
}
impl BlitPass {
    pub fn new(
        device: Arc<Device>,
        swapchain_image_format: Format,
        render_image: Arc<ImageView<StorageImage>>,
        render_image_sampler: Arc<Sampler>,
    ) -> Result<Self, RenderManagerError> {
        let pipeline = Self::create_pipeline(device.clone(), swapchain_image_format)?;
        let desc_set = Self::create_desc_set(
            pipeline.clone(),
            render_image.clone(),
            render_image_sampler.clone(),
        )?;
        Ok(Self { pipeline, desc_set })
    }

    /// Updates render image data e.g. when it has been resized
    pub fn update_render_image(
        &mut self,
        render_image: Arc<ImageView<StorageImage>>,
        render_image_sampler: Arc<Sampler>,
    ) -> Result<(), RenderManagerError> {
        self.desc_set =
            BlitPass::create_desc_set(self.pipeline.clone(), render_image, render_image_sampler)?;
        Ok(())
    }

    /// Records draw commands to a command buffer. Assumes that the command buffer is
    /// already in a render pass state, otherwise an error will be returned.
    pub fn record_commands(
        &self,
        command_buffer: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        viewport: Viewport,
    ) -> Result<(), DrawError> {
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
    fn create_pipeline(
        device: Arc<Device>,
        swapchain_image_format: Format,
    ) -> Result<Arc<GraphicsPipeline>, RenderManagerError> {
        let blit_vert_shader =
            create_shader_module(device.clone(), "assets/shader_binaries/blit.vert.spv")?;
        let blit_frag_shader =
            create_shader_module(device.clone(), "assets/shader_binaries/blit.frag.spv")?;

        GraphicsPipeline::start()
            .render_pass(PipelineRenderingCreateInfo {
                color_attachment_formats: vec![Some(swapchain_image_format)],
                ..Default::default()
            })
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .vertex_shader(
                blit_vert_shader
                    .entry_point("main")
                    .to_renderer_err("no main in blit.vert")?,
                (),
            )
            .fragment_shader(
                blit_frag_shader
                    .entry_point("main")
                    .to_renderer_err("no main in blit.frag")?,
                (),
            )
            .build(device.clone())
            .to_renderer_err("failed to create blit graphics pipeline")
    }

    fn create_desc_set(
        blit_pipeline: Arc<GraphicsPipeline>,
        render_image: Arc<ImageView<StorageImage>>,
        render_image_sampler: Arc<Sampler>,
    ) -> Result<Arc<PersistentDescriptorSet>, RenderManagerError> {
        PersistentDescriptorSet::new(
            blit_pipeline
                .layout()
                .set_layouts()
                .get(descriptor::SET_BLIT_FRAG)
                .unwrap()
                .to_owned(),
            [WriteDescriptorSet::image_view_sampler(
                descriptor::BINDING_SAMPLER,
                render_image,
                render_image_sampler,
            )],
        )
        .to_renderer_err("unable to create blit pass descriptor set")
    }
}
