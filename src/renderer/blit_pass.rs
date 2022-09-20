use super::render_manager::{create_shader_module, RenderManagerError, RenderManagerUnrecoverable};
use crate::shaders::shader_interfaces;
use std::sync::Arc;
use vulkano::{
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::Device,
    format::Format,
    image::{view::ImageView, StorageImage},
    pipeline::{
        graphics::{render_pass::PipelineRenderingCreateInfo, viewport::ViewportState},
        GraphicsPipeline, Pipeline,
    },
    sampler::Sampler,
};

pub struct BlitPass {
    pub pipeline: Arc<GraphicsPipeline>,
    pub desc_set: Arc<PersistentDescriptorSet>,
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

    pub fn create_desc_set(
        blit_pipeline: Arc<GraphicsPipeline>,
        render_image: Arc<ImageView<StorageImage>>,
        render_image_sampler: Arc<Sampler>,
    ) -> Result<Arc<PersistentDescriptorSet>, RenderManagerError> {
        PersistentDescriptorSet::new(
            blit_pipeline
                .layout()
                .set_layouts()
                .get(shader_interfaces::descriptor::SET_BLIT_FRAG)
                .unwrap()
                .to_owned(),
            [WriteDescriptorSet::image_view_sampler(
                shader_interfaces::descriptor::BINDING_SAMPLER,
                render_image,
                render_image_sampler,
            )],
        )
        .to_renderer_err("unable to create blit pass descriptor set")
    }
}
