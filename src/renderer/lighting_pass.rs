use super::{
    renderer_config::SHADER_ENTRY_POINT,
    shader_interfaces::uniform_buffers::CameraUniformBuffer,
    vulkan_helper::{create_shader_module, CreateDescriptorSetError, CreateShaderError},
};
use anyhow::Context;
#[allow(unused_imports)]
use log::{debug, error, info, warn};
use std::sync::Arc;
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    command_buffer::AutoCommandBufferBuilder,
    descriptor_set::{
        allocator::StandardDescriptorSetAllocator, PersistentDescriptorSet, WriteDescriptorSet,
    },
    device::Device,
    image::ImageViewAbstract,
    memory::allocator::StandardMemoryAllocator,
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
    pub const SET_G_BUFFERS: usize = 0;
    pub const BINDING_NORMAL: u32 = 0;
    pub const BINDING_PRIMITIVE_ID: u32 = 1;

    pub const SET_CAMERA: usize = 1;
    pub const BINDING_CAMERA: u32 = 0;
}

/// Defines functionality for reading the g-buffers and calculating the scene color values
pub struct LightingPass {
    pipeline: Arc<GraphicsPipeline>,

    desc_set_g_buffers: Arc<PersistentDescriptorSet>,
    desc_set_camera: Arc<PersistentDescriptorSet>,
}
// Public functions
impl LightingPass {
    pub fn new(
        device: Arc<Device>,
        descriptor_allocator: &StandardDescriptorSetAllocator,
        camera_buffer: Arc<CpuAccessibleBuffer<CameraUniformBuffer>>,
        g_buffer_normal: Arc<impl ImageViewAbstract + 'static>,
        g_buffer_primitive_id: Arc<impl ImageViewAbstract + 'static>,
        subpass: Subpass,
    ) -> anyhow::Result<Self> {
        let pipeline = create_pipeline(device.clone(), subpass)?;

        let desc_set_g_buffers = create_desc_set_gbuffers(
            descriptor_allocator,
            pipeline.clone(),
            g_buffer_normal,
            g_buffer_primitive_id,
        )?;

        let desc_set_camera =
            create_desc_set_camera(descriptor_allocator, pipeline.clone(), camera_buffer)?;

        Ok(Self {
            pipeline,
            desc_set_g_buffers,
            desc_set_camera,
        })
    }

    pub fn update_g_buffers(
        &mut self,
        descriptor_allocator: &StandardDescriptorSetAllocator,
        g_buffer_normal: Arc<impl ImageViewAbstract + 'static>,
        g_buffer_primitive_id: Arc<impl ImageViewAbstract + 'static>,
    ) -> anyhow::Result<()> {
        self.desc_set_g_buffers = create_desc_set_gbuffers(
            descriptor_allocator,
            self.pipeline.clone(),
            g_buffer_normal,
            g_buffer_primitive_id,
        )?;

        Ok(())
    }

    pub fn update_camera_ubo(&mut self, camera_data: CameraUniformBuffer) -> anyhow::Result<()> {
        todo!();

        Ok(())
    }

    /// Records draw commands to a command buffer. Assumes that the command buffer is
    /// already in a render pass state, otherwise an error will be returned.
    pub fn record_commands<L>(
        &self,
        command_buffer: &mut AutoCommandBufferBuilder<L>,
        viewport: Viewport,
    ) -> anyhow::Result<()> {
        let desc_sets = vec![
            self.desc_set_g_buffers.clone(),
            self.desc_set_camera.clone(),
        ];

        command_buffer
            .set_viewport(0, [viewport])
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                desc_sets,
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
                VERT_SHADER_PATH.to_owned(),
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
    g_buffer_normal: Arc<impl ImageViewAbstract + 'static>,
    g_buffer_primitive_id: Arc<impl ImageViewAbstract + 'static>,
) -> anyhow::Result<Arc<PersistentDescriptorSet>> {
    let set_layout = lighting_pipeline
        .layout()
        .set_layouts()
        .get(descriptor::SET_G_BUFFERS)
        .ok_or(CreateDescriptorSetError::InvalidDescriptorSetIndex {
            index: descriptor::SET_G_BUFFERS,
            shader_path: FRAG_SHADER_PATH,
        })?
        .to_owned();

    PersistentDescriptorSet::new(
        descriptor_allocator,
        set_layout,
        [
            WriteDescriptorSet::image_view(descriptor::BINDING_NORMAL, g_buffer_normal),
            WriteDescriptorSet::image_view(descriptor::BINDING_PRIMITIVE_ID, g_buffer_primitive_id),
        ],
    )
    .context("creating lighting pass g-buffer desc set")
}

fn create_desc_set_camera(
    descriptor_allocator: &StandardDescriptorSetAllocator,
    pipeline: Arc<GraphicsPipeline>,
    camera_buffer: Arc<CpuAccessibleBuffer<CameraUniformBuffer>>,
) -> anyhow::Result<Arc<PersistentDescriptorSet>> {
    let set_layout = pipeline
        .layout()
        .set_layouts()
        .get(descriptor::SET_CAMERA)
        .ok_or(CreateDescriptorSetError::InvalidDescriptorSetIndex {
            index: descriptor::SET_CAMERA,
            shader_path: FRAG_SHADER_PATH,
        })?
        .to_owned();

    PersistentDescriptorSet::new(
        descriptor_allocator,
        set_layout,
        [WriteDescriptorSet::buffer(
            descriptor::BINDING_CAMERA,
            camera_buffer,
        )],
    )
    .context("creating lighting pass camera desc set")
}
