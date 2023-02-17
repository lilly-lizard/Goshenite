use super::{
    config_renderer::SHADER_ENTRY_POINT, shader_interfaces::uniform_buffers::CameraUniformBuffer,
};
use anyhow::Context;
use ash::vk::{self};
use bort::{
    device::Device,
    pipeline_graphics::{
        ColorBlendState, DynamicState, GraphicsPipeline, GraphicsPipelineProperties,
    },
    shader_module::{ShaderModule, ShaderStage},
};
#[allow(unused_imports)]
use log::{debug, error, info, warn};
use std::{ffi::CString, sync::Arc};

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

fn create_pipeline(device: Arc<Device>) -> anyhow::Result<Arc<GraphicsPipeline>> {
    let vert_shader = Arc::new(
        ShaderModule::new_from_file(device.clone(), VERT_SHADER_PATH)
            .context("creating lighting pass vertex shader")?,
    );
    let vert_stage = ShaderStage::new(
        vk::ShaderStageFlags::VERTEX,
        vert_shader,
        CString::new(SHADER_ENTRY_POINT).context("shader entry point to c-string")?,
    );

    let frag_shader = Arc::new(
        ShaderModule::new_from_file(device.clone(), FRAG_SHADER_PATH)
            .context("creating lighting pass fragment shader")?,
    );
    let frag_stage = ShaderStage::new(
        vk::ShaderStageFlags::FRAGMENT,
        frag_shader,
        CString::new(SHADER_ENTRY_POINT).context("shader entry point to c-string")?,
    );

    let dynamic_state =
        DynamicState::new_default(vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);
    let color_blend_state =
        ColorBlendState::new_default(vec![vk::PipelineColorBlendAttachmentState {
            color_write_mask: vk::ColorComponentFlags::RGBA,
            ..Default::default()
        }]);

    let pipeline_properties = GraphicsPipelineProperties {
        color_blend_state,
        dynamic_state,
        ..Default::default()
    };

    todo!();
}
