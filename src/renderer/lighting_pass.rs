use super::{
    config_renderer::SHADER_ENTRY_POINT, shader_interfaces::uniform_buffers::CameraUniformBuffer,
};
use anyhow::Context;
use ash::vk;
use bort::{
    descriptor_layout::{
        DescriptorSetLayout, DescriptorSetLayoutBinding, DescriptorSetLayoutProperties,
    },
    descriptor_pool::{DescriptorPool, DescriptorPoolProperties},
    descriptor_set::DescriptorSet,
    device::Device,
    pipeline_graphics::{
        ColorBlendState, DynamicState, GraphicsPipeline, GraphicsPipelineProperties,
    },
    pipeline_layout::{PipelineLayout, PipelineLayoutProperties},
    render_pass::RenderPass,
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
    descriptor_pool: Arc<DescriptorPool>,
    desc_set_g_buffers: Arc<DescriptorSet>,
    pipeline_layout: Arc<PipelineLayout>,
    pipeline: Arc<GraphicsPipeline>,
}

impl LightingPass {
    pub fn new(
        device: Arc<Device>,
        desc_set_layout_camera: Arc<DescriptorSetLayout>,
        render_pass: &RenderPass,
        subpass_index: u32,
    ) -> anyhow::Result<Self> {
        let descriptor_pool = create_descriptor_pool(device.clone())?;
        let desc_set_g_buffers = create_desc_set_gbuffers(device.clone(), descriptor_pool.clone())?;

        let pipeline_layout = create_pipeline_layout(
            device.clone(),
            desc_set_layout_camera,
            desc_set_g_buffers.layout().clone(),
        )?;
        let pipeline = create_pipeline(
            device.clone(),
            pipeline_layout.clone(),
            render_pass,
            subpass_index,
        )?;

        Ok(Self {
            descriptor_pool,
            desc_set_g_buffers,
            pipeline_layout,
            pipeline,
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

fn create_descriptor_pool(device: Arc<Device>) -> anyhow::Result<Arc<DescriptorPool>> {
    let descriptor_pool_props = DescriptorPoolProperties {
        max_sets: 8,
        pool_sizes: vec![vk::DescriptorPoolSize {
            ty: vk::DescriptorType::INPUT_ATTACHMENT,
            descriptor_count: 2,
        }],
        ..Default::default()
    };

    let descriptor_pool = DescriptorPool::new(device, descriptor_pool_props)
        .context("creating lighting pass descriptor pool")?;

    Ok(Arc::new(descriptor_pool))
}

fn create_desc_set_gbuffers(
    device: Arc<Device>,
    descriptor_pool: Arc<DescriptorPool>,
) -> anyhow::Result<Arc<DescriptorSet>> {
    let mut desc_set_layout_props = DescriptorSetLayoutProperties::default();
    desc_set_layout_props.bindings = vec![
        DescriptorSetLayoutBinding {
            binding: descriptor::BINDING_NORMAL,
            descriptor_type: vk::DescriptorType::INPUT_ATTACHMENT,
            descriptor_count: 1,
            stage_flags: vk::ShaderStageFlags::FRAGMENT,
        },
        DescriptorSetLayoutBinding {
            binding: descriptor::BINDING_PRIMITIVE_ID,
            descriptor_type: vk::DescriptorType::INPUT_ATTACHMENT,
            descriptor_count: 1,
            stage_flags: vk::ShaderStageFlags::FRAGMENT,
        },
    ];

    let desc_set_layout = Arc::new(
        DescriptorSetLayout::new(device, desc_set_layout_props)
            .context("creating lighting pass g-buffer descriptor set layout")?,
    );

    let desc_set = descriptor_pool
        .allocate_descriptor_set(desc_set_layout)
        .context("allocating lighting pass g-buffer descriptor set")?;

    Ok(Arc::new(desc_set))
}

fn create_pipeline_layout(
    device: Arc<Device>,
    desc_set_layout_camera: Arc<DescriptorSetLayout>,
    desc_set_layout_g_buffers: Arc<DescriptorSetLayout>,
) -> anyhow::Result<Arc<PipelineLayout>> {
    let mut pipeline_layout_props = PipelineLayoutProperties::default();
    pipeline_layout_props.set_layouts = vec![desc_set_layout_camera, desc_set_layout_g_buffers];

    let pipeline_layout = PipelineLayout::new(device, pipeline_layout_props)
        .context("creating lighting pass pipeline layout")?;

    Ok(Arc::new(pipeline_layout))
}

fn create_pipeline(
    device: Arc<Device>,
    pipeline_layout: Arc<PipelineLayout>,
    render_pass: &RenderPass,
    subpass_index: u32,
) -> anyhow::Result<Arc<GraphicsPipeline>> {
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

    let shader_stages = vec![vert_stage, frag_stage];

    let dynamic_state =
        DynamicState::new_default(vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);
    let color_blend_state =
        ColorBlendState::new_default(vec![vk::PipelineColorBlendAttachmentState {
            color_write_mask: vk::ColorComponentFlags::RGBA,
            ..Default::default()
        }]);

    let mut pipeline_properties = GraphicsPipelineProperties::default();
    pipeline_properties.dynamic_state = dynamic_state;
    pipeline_properties.color_blend_state = color_blend_state;

    let pipeline = GraphicsPipeline::new(
        pipeline_layout,
        pipeline_properties,
        shader_stages,
        render_pass,
        subpass_index,
        None,
    )
    .context("creating lighting pass pipeline")?;

    Ok(Arc::new(pipeline))
}
