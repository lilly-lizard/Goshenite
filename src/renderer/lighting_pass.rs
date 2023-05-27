use super::{
    config_renderer::SHADER_ENTRY_POINT, shader_interfaces::uniform_buffers::CameraUniformBuffer,
    vulkan_init::render_pass_indices,
};
use anyhow::Context;
use ash::vk;
use bort::{
    Buffer, ColorBlendState, CommandBuffer, DescriptorPool, DescriptorPoolProperties,
    DescriptorSet, DescriptorSetLayout, DescriptorSetLayoutBinding, DescriptorSetLayoutProperties,
    Device, DeviceOwned, DynamicState, GraphicsPipeline, GraphicsPipelineProperties, Image,
    ImageView, ImageViewAccess, PipelineAccess, PipelineLayout, PipelineLayoutProperties,
    RenderPass, ShaderModule, ShaderStage, ViewportState,
};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::{ffi::CString, mem, sync::Arc};

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
    device: Arc<Device>,

    desc_set_camera: Arc<DescriptorSet>,
    /// One per framebuffer
    desc_sets_g_buffer: Vec<Arc<DescriptorSet>>,

    pipeline: Arc<GraphicsPipeline>,
}

impl LightingPass {
    pub fn new(
        device: Arc<Device>,
        render_pass: &RenderPass,
        camera_buffer: &Buffer,
        normal_buffer: &ImageView<Image>,
        primitive_id_buffers: &Vec<Arc<ImageView<Image>>>,
    ) -> anyhow::Result<Self> {
        let descriptor_pool = create_descriptor_pool(device.clone())?;

        let desc_set_camera = create_desc_set_camera(descriptor_pool.clone())?;
        write_desc_set_camera(&desc_set_camera, camera_buffer)?;

        let desc_sets_g_buffer =
            create_desc_sets_gbuffer(descriptor_pool.clone(), primitive_id_buffers.len())?;
        write_desc_sets_gbuffer(&desc_sets_g_buffer, normal_buffer, primitive_id_buffers)?;

        let pipeline_layout = create_pipeline_layout(
            device.clone(),
            desc_set_camera.layout().clone(),
            desc_sets_g_buffer[0].layout().clone(),
        )?;

        let pipeline = create_pipeline(device.clone(), pipeline_layout.clone(), render_pass)?;

        Ok(Self {
            device,
            desc_sets_g_buffer,
            desc_set_camera,
            pipeline,
        })
    }

    pub fn update_g_buffers(
        &mut self,
        normal_buffer: &ImageView<Image>,
        primitive_id_buffers: &Vec<Arc<ImageView<Image>>>,
    ) -> anyhow::Result<()> {
        write_desc_sets_gbuffer(
            &self.desc_sets_g_buffer,
            normal_buffer,
            primitive_id_buffers,
        )
    }

    pub fn update_camera_descriptor_set(&self, camera_buffer: &Buffer) -> anyhow::Result<()> {
        write_desc_set_camera(&self.desc_set_camera, camera_buffer)
    }

    /// Records draw commands to a command buffer.
    ///
    /// **Assumes that the command buffer is already in a render pass state.**
    pub fn record_commands(
        &self,
        frame_index: usize,
        command_buffer: &CommandBuffer,
        viewport: vk::Viewport,
        scissor: vk::Rect2D,
    ) -> anyhow::Result<()> {
        let device_ash = self.device.inner();
        let command_buffer_handle = command_buffer.handle();
        let descriptor_set_handles = [
            self.desc_sets_g_buffer[frame_index].handle(),
            self.desc_set_camera.handle(),
        ];

        unsafe {
            device_ash.cmd_bind_pipeline(
                command_buffer_handle,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline.handle(),
            );
            device_ash.cmd_set_viewport(command_buffer_handle, 0, &[viewport]);
            device_ash.cmd_set_scissor(command_buffer_handle, 0, &[scissor]);
            device_ash.cmd_bind_descriptor_sets(
                command_buffer_handle,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline.pipeline_layout().handle(),
                0,
                &descriptor_set_handles,
                &[],
            );
            device_ash.cmd_draw(command_buffer_handle, 3, 1, 0, 0);
        }

        Ok(())
    }
}

fn create_descriptor_pool(device: Arc<Device>) -> anyhow::Result<Arc<DescriptorPool>> {
    let descriptor_pool_props = DescriptorPoolProperties {
        max_sets: 3,
        pool_sizes: vec![
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::INPUT_ATTACHMENT,
                descriptor_count: 2,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
            },
        ],
        ..Default::default()
    };

    let descriptor_pool = DescriptorPool::new(device, descriptor_pool_props)
        .context("creating lighting pass descriptor pool")?;

    Ok(Arc::new(descriptor_pool))
}

fn create_desc_set_camera(
    descriptor_pool: Arc<DescriptorPool>,
) -> anyhow::Result<Arc<DescriptorSet>> {
    let desc_set_layout_props =
        DescriptorSetLayoutProperties::new_default(vec![DescriptorSetLayoutBinding {
            binding: descriptor::BINDING_CAMERA,
            descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: 1,
            stage_flags: vk::ShaderStageFlags::FRAGMENT,
            ..Default::default()
        }]);

    let desc_set_layout = Arc::new(
        DescriptorSetLayout::new(descriptor_pool.device().clone(), desc_set_layout_props)
            .context("creating lighting pass camera descriptor set layout")?,
    );

    let desc_set = descriptor_pool
        .allocate_descriptor_set(desc_set_layout)
        .context("allocating lighting pass camera descriptor set")?;

    Ok(Arc::new(desc_set))
}

fn write_desc_set_camera(
    desc_set_camera: &DescriptorSet,
    camera_buffer: &Buffer,
) -> anyhow::Result<()> {
    let camera_buffer_info = vk::DescriptorBufferInfo {
        buffer: camera_buffer.handle(),
        offset: 0,
        range: mem::size_of::<CameraUniformBuffer>() as vk::DeviceSize,
    };

    let descriptor_writes = [vk::WriteDescriptorSet::builder()
        .dst_set(desc_set_camera.handle())
        .dst_binding(descriptor::BINDING_CAMERA)
        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
        .buffer_info(&[camera_buffer_info])
        .build()];

    unsafe {
        desc_set_camera
            .device()
            .inner()
            .update_descriptor_sets(&descriptor_writes, &[]);
    }

    Ok(())
}

fn create_desc_sets_gbuffer(
    descriptor_pool: Arc<DescriptorPool>,
    framebuffer_count: usize,
) -> anyhow::Result<Vec<Arc<DescriptorSet>>> {
    (0..framebuffer_count)
        .into_iter()
        .map(|_| create_desc_set_gbuffer(descriptor_pool.clone()))
        .collect::<anyhow::Result<Vec<_>>>()
}

fn create_desc_set_gbuffer(
    descriptor_pool: Arc<DescriptorPool>,
) -> anyhow::Result<Arc<DescriptorSet>> {
    let mut desc_set_layout_props = DescriptorSetLayoutProperties::default();
    desc_set_layout_props.bindings = vec![
        DescriptorSetLayoutBinding {
            binding: descriptor::BINDING_NORMAL,
            descriptor_type: vk::DescriptorType::INPUT_ATTACHMENT,
            descriptor_count: 1,
            stage_flags: vk::ShaderStageFlags::FRAGMENT,
            ..Default::default()
        },
        DescriptorSetLayoutBinding {
            binding: descriptor::BINDING_PRIMITIVE_ID,
            descriptor_type: vk::DescriptorType::INPUT_ATTACHMENT,
            descriptor_count: 1,
            stage_flags: vk::ShaderStageFlags::FRAGMENT,
            ..Default::default()
        },
    ];

    let desc_set_layout = Arc::new(
        DescriptorSetLayout::new(descriptor_pool.device().clone(), desc_set_layout_props)
            .context("creating lighting pass g-buffer descriptor set layout")?,
    );

    let desc_set = descriptor_pool
        .allocate_descriptor_set(desc_set_layout)
        .context("allocating lighting pass g-buffer descriptor set")?;

    Ok(Arc::new(desc_set))
}

fn write_desc_sets_gbuffer(
    desc_sets_gbuffer: &Vec<Arc<DescriptorSet>>,
    normal_buffer: &ImageView<Image>,
    primitive_id_buffers: &Vec<Arc<ImageView<Image>>>,
) -> anyhow::Result<()> {
    for i in 0..desc_sets_gbuffer.len() {
        write_desc_set_gbuffer(
            desc_sets_gbuffer[i].as_ref(),
            normal_buffer,
            primitive_id_buffers[i].as_ref(),
        )?;
    }
    Ok(())
}

fn write_desc_set_gbuffer(
    desc_set_gbuffer: &DescriptorSet,
    normal_buffer: &impl ImageViewAccess,
    primitive_id_buffer: &impl ImageViewAccess,
) -> anyhow::Result<()> {
    let normal_buffer_info = vk::DescriptorImageInfo {
        image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        image_view: normal_buffer.handle(),
        ..Default::default()
    };

    let primitive_id_buffer_info = vk::DescriptorImageInfo {
        image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        image_view: primitive_id_buffer.handle(),
        ..Default::default()
    };

    let descriptor_writes = [
        vk::WriteDescriptorSet::builder()
            .dst_set(desc_set_gbuffer.handle())
            .dst_binding(descriptor::BINDING_NORMAL)
            .descriptor_type(vk::DescriptorType::INPUT_ATTACHMENT)
            .image_info(&[normal_buffer_info])
            .build(),
        vk::WriteDescriptorSet::builder()
            .dst_set(desc_set_gbuffer.handle())
            .dst_binding(descriptor::BINDING_PRIMITIVE_ID)
            .descriptor_type(vk::DescriptorType::INPUT_ATTACHMENT)
            .image_info(&[primitive_id_buffer_info])
            .build(),
    ];

    unsafe {
        desc_set_gbuffer
            .device()
            .inner()
            .update_descriptor_sets(&descriptor_writes, &[]);
    }

    Ok(())
}

fn create_pipeline_layout(
    device: Arc<Device>,
    desc_set_layout_camera: Arc<DescriptorSetLayout>,
    desc_set_layout_g_buffers: Arc<DescriptorSetLayout>,
) -> anyhow::Result<Arc<PipelineLayout>> {
    let pipeline_layout_props = PipelineLayoutProperties::new(
        vec![desc_set_layout_g_buffers, desc_set_layout_camera],
        Vec::new(),
    );

    let pipeline_layout = PipelineLayout::new(device, pipeline_layout_props)
        .context("creating lighting pass pipeline layout")?;

    Ok(Arc::new(pipeline_layout))
}

fn create_pipeline(
    device: Arc<Device>,
    pipeline_layout: Arc<PipelineLayout>,
    render_pass: &RenderPass,
) -> anyhow::Result<Arc<GraphicsPipeline>> {
    let (vert_stage, frag_stage) = create_shader_stages(&device)?;

    let dynamic_state =
        DynamicState::new_default(vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);

    let viewport_state = ViewportState::new_dynamic(1, 1);

    let color_blend_state =
        ColorBlendState::new_default(vec![ColorBlendState::blend_state_disabled()]);

    let mut pipeline_properties = GraphicsPipelineProperties::default();
    pipeline_properties.subpass_index = render_pass_indices::SUBPASS_DEFERRED as u32;
    pipeline_properties.dynamic_state = dynamic_state;
    pipeline_properties.color_blend_state = color_blend_state;
    pipeline_properties.viewport_state = viewport_state;

    let pipeline = GraphicsPipeline::new(
        pipeline_layout,
        pipeline_properties,
        &[vert_stage, frag_stage],
        render_pass,
        None,
    )
    .context("creating lighting pass pipeline")?;

    Ok(Arc::new(pipeline))
}

#[cfg(feature = "include-spirv-bytes")]
fn create_shader_stages(device: &Arc<Device>) -> anyhow::Result<(ShaderStage, ShaderStage)> {
    let mut vertex_spv_file = std::io::Cursor::new(
        &include_bytes!("../../assets/shader_binaries/full_screen.vert.spv")[..],
    );
    let vert_shader = Arc::new(
        ShaderModule::new_from_spirv(device.clone(), &mut vertex_spv_file)
            .context("creating lighting pass vertex shader")?,
    );
    let vert_stage = ShaderStage::new(
        vk::ShaderStageFlags::VERTEX,
        vert_shader,
        CString::new(SHADER_ENTRY_POINT).context("converting shader entry point to c-string")?,
    );

    let mut frag_spv_file = std::io::Cursor::new(
        &include_bytes!("../../assets/shader_binaries/scene_lighting.frag.spv")[..],
    );
    let frag_shader = Arc::new(
        ShaderModule::new_from_spirv(device.clone(), &mut frag_spv_file)
            .context("creating lighting pass fragment shader")?,
    );
    let frag_stage = ShaderStage::new(
        vk::ShaderStageFlags::FRAGMENT,
        frag_shader,
        CString::new(SHADER_ENTRY_POINT).context("converting shader entry point to c-string")?,
    );

    Ok((vert_stage, frag_stage))
}

#[cfg(not(feature = "include-spirv-bytes"))]
fn create_shader_stages(device: &Arc<Device>) -> anyhow::Result<(ShaderStage, ShaderStage)> {
    const VERT_SHADER_PATH: &str = "assets/shader_binaries/full_screen.vert.spv";
    const FRAG_SHADER_PATH: &str = "assets/shader_binaries/scene_lighting.frag.spv";

    let vert_shader = Arc::new(
        ShaderModule::new_from_file(device.clone(), VERT_SHADER_PATH)
            .context("creating lighting pass vertex shader")?,
    );
    let vert_stage = ShaderStage::new(
        vk::ShaderStageFlags::VERTEX,
        vert_shader,
        CString::new(SHADER_ENTRY_POINT).context("converting shader entry point to c-string")?,
    );

    let frag_shader = Arc::new(
        ShaderModule::new_from_file(device.clone(), FRAG_SHADER_PATH)
            .context("creating lighting pass fragment shader")?,
    );
    let frag_stage = ShaderStage::new(
        vk::ShaderStageFlags::FRAGMENT,
        frag_shader,
        CString::new(SHADER_ENTRY_POINT).context("converting shader entry point to c-string")?,
    );

    Ok((vert_stage, frag_stage))
}

impl Drop for LightingPass {
    fn drop(&mut self) {
        trace!("dropping lighting pass...");
    }
}
