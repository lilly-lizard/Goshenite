use super::{
    config_renderer::SHADER_ENTRY_POINT, shader_interfaces::uniform_buffers::CameraUniformBuffer,
};
use anyhow::Context;
use ash::vk;
#[allow(unused_imports)]
use log::{debug, error, info, warn};
use std::sync::Arc;

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
    pipeline: Arc<vk::Pipeline>,
    descriptor_pool: Arc<vk::DescriptorPool>,
    desc_set_g_buffers: Arc<vk::DescriptorSet>,
}

impl LightingPass {
    pub fn new(device: Arc<vk::Device>) -> anyhow::Result<Self> {
        let pipeline = todo!("create_pipeline");
        let descriptor_pool = todo!("descriptor pool");
        let desc_set_g_buffers = todo!("create_desc_set_gbuffers");

        Ok(Self {
            pipeline,
            descriptor_pool,
            desc_set_g_buffers,
        })
    }

    pub fn update_g_buffers(&mut self) -> anyhow::Result<()> {
        todo!("create_desc_set_gbuffers");

        Ok(())
    }

    /// Records draw commands to a command buffer. Assumes that the command buffer is
    /// already in a render pass state, otherwise an error will be returned.
    pub fn record_commands<L>(
        &self,
        //command_buffer: &mut AutoCommandBufferBuilder<L>,
        //viewport: vk::Viewport,
        //camera_buffer: Arc<CpuAccessibleBuffer<CameraUniformBuffer>>,
    ) -> anyhow::Result<()> {
        todo!("create_desc_set_camera");

        Ok(())
    }
}

fn create_pipeline(device: Arc<vk::Device>, subpass: Subpass) -> anyhow::Result<Arc<vk::Pipeline>> {
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
