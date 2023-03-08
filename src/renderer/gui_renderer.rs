/// shout out to https://github.com/hakolao/egui_winit_vulkano for a lot of this code
use super::{
    config_renderer::SHADER_ENTRY_POINT,
    shader_interfaces::{push_constants::GuiPushConstant, vertex_inputs::EguiVertex},
};
use crate::user_interface::gui::Gui;
use ahash::AHashMap;
use anyhow::Context;
use ash::vk;
use bort::{
    buffer::{Buffer, BufferProperties},
    command_buffer::CommandBuffer,
    command_pool::{CommandPool, CommandPoolProperties},
    descriptor_layout::{
        DescriptorSetLayout, DescriptorSetLayoutBinding, DescriptorSetLayoutProperties,
    },
    descriptor_pool::{DescriptorPool, DescriptorPoolProperties},
    descriptor_set::DescriptorSet,
    device::Device,
    image::Image,
    image_access::{ImageAccess, ImageViewAccess},
    image_properties::{ImageDimensions, ImageProperties},
    image_view::{default_subresource_layers, ImageView, ImageViewProperties},
    memory::{cpu_accessible_allocation_info, MemoryAllocator},
    pipeline_access::PipelineAccess,
    pipeline_graphics::{
        ColorBlendState, DynamicState, GraphicsPipeline, GraphicsPipelineProperties,
    },
    pipeline_layout::{PipelineLayout, PipelineLayoutProperties},
    queue::Queue,
    render_pass::RenderPass,
    sampler::{Sampler, SamplerProperties},
    semaphore::Semaphore,
    shader_module::{ShaderModule, ShaderStage},
};
use egui::{epaint::Primitive, ClippedPrimitive, Mesh, Rect, TextureId, TexturesDelta};
#[allow(unused_imports)]
use log::{debug, error, info, warn};
use std::{
    ffi::CString,
    fmt::{self, Display},
    sync::Arc,
};
use vk_mem::AllocationCreateInfo;

const VERT_SHADER_PATH: &str = "assets/shader_binaries/gui.vert.spv";
const FRAG_SHADER_PATH: &str = "assets/shader_binaries/gui.frag.spv";

const VERTICES_PER_QUAD: vk::DeviceSize = 4;
const VERTEX_BUFFER_SIZE: vk::DeviceSize = 1024 * 1024 * VERTICES_PER_QUAD;
const INDEX_BUFFER_SIZE: vk::DeviceSize = 1024 * 1024 * 2;

const TEXTURE_FORMAT: vk::Format = vk::Format::R8G8B8A8_SRGB;

const MAX_DESC_SETS_PER_POOL: u32 = 64;

mod descriptor {
    pub const SET_FONT_TEXTURE: usize = 0;
    pub const BINDING_FONT_TEXTURE: u32 = 0;
}

/// Index format
type VertexIndex = u32;

pub struct GuiRenderer {
    device: Arc<Device>,

    memory_allocator: Arc<MemoryAllocator>,
    transient_command_pool: Arc<CommandPool>,
    pipeline: Arc<GraphicsPipeline>,

    descriptor_pools: Vec<Arc<DescriptorPool>>,
    unused_texture_desc_sets: Vec<Arc<DescriptorSet>>,

    texture_sampler: Arc<Sampler>,
    texture_image_views: AHashMap<egui::TextureId, Arc<ImageView<Image>>>,
    texture_desc_sets: AHashMap<egui::TextureId, Arc<DescriptorSet>>,
}

// Public functions

impl GuiRenderer {
    /// Initializes the gui renderer
    pub fn new(
        device: Arc<Device>,
        queue_family_index: u32,
        memory_allocator: Arc<MemoryAllocator>,
        render_pass: &RenderPass,
        subpass_index: u32,
    ) -> anyhow::Result<Self> {
        let transient_command_pool =
            create_transient_command_pool(device.clone(), queue_family_index)?;

        let desc_set_layout = create_descriptor_layout(device.clone())?;

        let pipeline_layout = create_pipeline_layout(device.clone(), desc_set_layout)?;
        let pipeline =
            create_pipeline(device.clone(), pipeline_layout, render_pass, subpass_index)?;

        let texture_sampler = create_texture_sampler(device.clone())?;

        let descriptor_pool = create_descriptor_pool(device.clone())?;

        Ok(Self {
            device,
            memory_allocator,
            transient_command_pool,
            pipeline,

            descriptor_pools: vec![descriptor_pool],
            unused_texture_desc_sets: Vec::new(),

            texture_sampler,
            texture_image_views: AHashMap::default(),
            texture_desc_sets: AHashMap::default(),
        })
    }

    /// Creates and/or removes texture resources as required by [`TexturesDelta`](epaint::Textures::TexturesDelta)
    /// output by [`egui::end_frame`](egui::context::Context::end_frame). If new textures were created, submits a
    /// command buffer and returns a signal semaphore for the submission.
    pub fn update_textures(
        &mut self,
        textures_delta_vec: Vec<TexturesDelta>,
        queue: &Queue,
        wait_semaphores: Vec<Semaphore>,
    ) -> anyhow::Result<Option<Semaphore>> {
        // return if empty
        if textures_delta_vec.is_empty() {
            return Ok(None);
        }

        // create command buffer
        let command_buffer = CommandBuffer::new(
            self.transient_command_pool.clone(),
            vk::CommandBufferLevel::PRIMARY,
        )
        .context("creating command buffer for gui texture upload")?;

        let mut commands_recorded = false;

        for textures_delta in textures_delta_vec {
            // release unused texture resources
            for &id in &textures_delta.free {
                self.unregister_image(id);
            }

            // create new images and record upload commands
            for (id, image_delta) in textures_delta.set {
                self.create_texture(id, image_delta, &command_buffer, queue)?;
                commands_recorded = true;
            }
        }

        // execute command buffer
        if commands_recorded {
            let (wait_semaphore_handles, wait_semaphore_stages): (Vec<_>, Vec<_>) = wait_semaphores
                .iter()
                .map(|semaphore| (semaphore.handle(), vk::PipelineStageFlags::TRANSFER))
                .unzip();

            let signal_semaphore =
                Semaphore::new(self.device.clone()).context("creating texture upload semaphore")?;
            let signal_semaphore_handles = [signal_semaphore.handle()];

            let command_buffer_handles = [command_buffer.handle()];

            let submit_info = vk::SubmitInfo::builder()
                .wait_semaphores(wait_semaphore_handles.as_slice())
                .wait_dst_stage_mask(wait_semaphore_stages.as_slice())
                .signal_semaphores(&signal_semaphore_handles)
                .command_buffers(&command_buffer_handles);

            queue
                .submit(&[*submit_info], None)
                .context("submitting gui texture upload commands")?;

            return Ok(Some(signal_semaphore));
        }

        Ok(None)
    }

    /// Record gui rendering commands
    /// * `command_buffer`: Primary command buffer to record commands to. Must be already in dynamic rendering state.
    /// * `primitives`: List of egui primitives to render. Can aquire from [Gui::primitives](`crate::gui::Gui::primitives`).
    /// * `scale_factor`: Gui dpi config. Can aquire from [Gui::scale_factor](`crate::gui::Gui::scale_factor`).
    /// * `is_srgb_framebuffer`: Set to true if rendering to an SRGB framebuffer.
    /// * `framebuffer_dimensions`: Framebuffer dimensions.
    pub(super) fn record_commands<L>(
        &mut self,
        gui: &Gui,
        is_srgb_framebuffer: bool,
        framebuffer_dimensions: [f32; 2],
    ) -> anyhow::Result<()> {
        todo!()
    }
}

// Private functions

impl GuiRenderer {
    // TODO EXTRACT SUBFUNCTIONS
    /// Creates a new texture needing to be added for the gui.
    ///
    /// Helper function for [`GuiRenderer::update_textures`]
    fn create_texture(
        &mut self,
        texture_id: egui::TextureId,
        delta: egui::epaint::ImageDelta,
        command_buffer: &CommandBuffer,
        queue: &Queue,
    ) -> anyhow::Result<()> {
        // extract pixel data from egui
        let data: Vec<u8> = match &delta.image {
            egui::ImageData::Color(image) => {
                if image.width() * image.height() != image.pixels.len() {
                    warn!(
                        "mismatch between gui texture size and texel count, skipping... texture_id = {:?}",
                        texture_id
                    );
                    return Ok(());
                }
                image
                    .pixels
                    .iter()
                    .flat_map(|color| color.to_array())
                    .collect()
            }
            egui::ImageData::Font(image) => image
                .srgba_pixels(None)
                .flat_map(|color| color.to_array())
                .collect(),
        };
        if data.len() == 0 {
            warn!(
                "attempted to create gui texture with no data! skipping... texture_id = {:?}",
                texture_id
            );
            return Ok(());
        }

        // create buffer to be copied to the image
        let mut texture_data_buffer = create_texture_data_buffer(
            self.memory_allocator.clone(),
            std::mem::size_of_val(&data) as u64,
        )?;
        texture_data_buffer
            .write_iter(data, 0)
            .context("uploading gui texture data to staging buffer")?;

        if let Some(update_pos) = delta.pos {
            // sometimes a subregion of an already allocated texture needs to be updated e.g. when a font size is changed
            if let Some(existing_image_view) = self.texture_image_views.get(&texture_id) {
                let copy_region = vk::BufferImageCopy {
                    image_subresource: default_subresource_layers(vk::ImageAspectFlags::COLOR),
                    image_offset: vk::Offset3D {
                        x: update_pos[0] as i32,
                        y: update_pos[1] as i32,
                        z: 0,
                    },
                    image_extent: vk::Extent3D {
                        width: delta.image.width() as u32,
                        height: delta.image.height() as u32,
                        depth: 1,
                    },
                    ..Default::default()
                };
                debug!("updating existing gui texture id = {:?}, region offset = {:?}, region extent = {:?}",
                    texture_id, copy_region.image_offset, copy_region.image_extent);

                // we need to transition the image layout to vk::ImageLayout::GENERAL
                let to_general_image_barrier = vk::ImageMemoryBarrier::builder()
                    .src_access_mask(vk::AccessFlags::SHADER_READ)
                    .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                    .old_layout(vk::ImageLayout::UNDEFINED)
                    .new_layout(vk::ImageLayout::GENERAL)
                    .image(existing_image_view.image().handle())
                    .subresource_range(existing_image_view.properties().subresource_range);

                // then transition back to vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
                let to_shader_read_image_barrier = vk::ImageMemoryBarrier::builder()
                    .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                    .dst_access_mask(vk::AccessFlags::SHADER_READ)
                    .old_layout(vk::ImageLayout::GENERAL)
                    .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .image(existing_image_view.image().handle())
                    .subresource_range(existing_image_view.properties().subresource_range);

                // copy buffer to image
                unsafe {
                    self.device.inner().cmd_pipeline_barrier(
                        command_buffer.handle(),
                        vk::PipelineStageFlags::FRAGMENT_SHADER,
                        vk::PipelineStageFlags::TRANSFER,
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &[*to_general_image_barrier],
                    );

                    self.device.inner().cmd_copy_buffer_to_image(
                        command_buffer.handle(),
                        texture_data_buffer.handle(),
                        existing_image_view.image().handle(),
                        vk::ImageLayout::GENERAL,
                        &[copy_region],
                    );

                    self.device.inner().cmd_pipeline_barrier(
                        command_buffer.handle(),
                        vk::PipelineStageFlags::TRANSFER,
                        vk::PipelineStageFlags::FRAGMENT_SHADER,
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &[*to_shader_read_image_barrier],
                    );
                }
            }
        } else {
            // usually ImageDelta.pos == None meaning a new image needs to be created
            debug!("creating new gui texture id = {:?}", texture_id);

            // create image

            let new_image_properties = ImageProperties::new_default(
                TEXTURE_FORMAT,
                ImageDimensions::new_2d(delta.image.width() as u32, delta.image.height() as u32),
                vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            );

            let new_image_allocation_info = AllocationCreateInfo {
                required_flags: vk::MemoryPropertyFlags::DEVICE_LOCAL,
                ..Default::default()
            };

            let new_image = Arc::new(
                Image::new(
                    self.memory_allocator.clone(),
                    new_image_properties.clone(),
                    new_image_allocation_info,
                )
                .context("creating image for new egui texture")?,
            );

            // create image view

            let new_image_view_properties =
                ImageViewProperties::from_image_properties_default(&new_image_properties);
            let new_image_view = Arc::new(
                ImageView::new(new_image.clone(), new_image_view_properties)
                    .context("creating image view for new egui texture")?,
            );

            // copy buffer to image
            let copy_region = vk::BufferImageCopy {
                image_subresource: default_subresource_layers(vk::ImageAspectFlags::COLOR),
                image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
                image_extent: vk::Extent3D {
                    width: delta.image.width() as u32,
                    height: delta.image.height() as u32,
                    depth: 1,
                },
                ..Default::default()
            };
            debug!("updating existing gui texture id = {:?}, region offset = {:?}, region extent = {:?}",
                texture_id, copy_region.image_offset, copy_region.image_extent);

            // after uploading we need to transfer to vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
            let to_shader_read_image_barrier = vk::ImageMemoryBarrier::builder()
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ)
                .old_layout(vk::ImageLayout::GENERAL)
                .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image(new_image_view.image().handle())
                .subresource_range(new_image_view.properties().subresource_range);

            // copy buffer to image
            unsafe {
                self.device.inner().cmd_pipeline_barrier(
                    command_buffer.handle(),
                    vk::PipelineStageFlags::FRAGMENT_SHADER,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[],
                );

                self.device.inner().cmd_copy_buffer_to_image(
                    command_buffer.handle(),
                    texture_data_buffer.handle(),
                    new_image.handle(),
                    vk::ImageLayout::GENERAL,
                    &[copy_region],
                );

                self.device.inner().cmd_pipeline_barrier(
                    command_buffer.handle(),
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::FRAGMENT_SHADER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[*to_shader_read_image_barrier],
                );
            }

            // create new descriptor set
            let font_desc_set = self.get_new_font_texture_desc_set()?;

            // write descriptor set
            write_font_texture_desc_set(
                &self.device,
                &font_desc_set,
                &new_image_view,
                &self.texture_sampler,
            )?;

            // store new texture
            self.texture_desc_sets.insert(texture_id, font_desc_set);
            self.texture_image_views.insert(texture_id, new_image_view);
        }

        Ok(())
    }

    /// Unregister a texture that is no longer required by the gui.
    ///
    /// Helper function for [`Self::update_textures`]
    fn unregister_image(&mut self, texture_id: egui::TextureId) {
        debug!("removing unneeded gui texture id = {:?}", texture_id);
        self.texture_image_views.remove(&texture_id);
        let unused_desc_set = self.texture_desc_sets.remove(&texture_id);
        if let Some(unused_desc_set) = unused_desc_set {
            self.unused_texture_desc_sets.push(unused_desc_set);
        }
    }

    fn get_new_font_texture_desc_set(&mut self) -> anyhow::Result<Arc<DescriptorSet>> {
        if let Some(existing_desc_set) = self.unused_texture_desc_sets.pop() {
            // reuse old desc set
            return Ok(existing_desc_set);
        }
        // else allocate new one
        return self.allocate_font_texture_desc_set();
    }

    fn allocate_font_texture_desc_set(&mut self) -> anyhow::Result<Arc<DescriptorSet>> {
        let set_layout = self
            .pipeline
            .pipeline_layout()
            .properties()
            .set_layouts
            .get(descriptor::SET_FONT_TEXTURE)
            .context("indexing gui pipeline descriptor set layout")?;

        let allocate_res = self
            .descriptor_pools
            .get(self.descriptor_pools.len() - 1)
            .expect("should always be at least one descriptor pool")
            .allocate_descriptor_set(set_layout.clone());

        let desc_set = match allocate_res {
            Err(allocate_error) => match allocate_error {
                vk::Result::ERROR_OUT_OF_POOL_MEMORY | vk::Result::ERROR_FRAGMENTED_POOL => {
                    self.allocate_font_texture_desc_set_from_new_pool(set_layout.clone())?
                }
                _ => {
                    return Err(allocate_error)
                        .context("allocating descriptor set for new egui texture")
                }
            },
            Ok(desc_set) => desc_set,
        };

        Ok(Arc::new(desc_set))
    }

    fn allocate_font_texture_desc_set_from_new_pool(
        &mut self,
        set_layout: Arc<DescriptorSetLayout>,
    ) -> anyhow::Result<DescriptorSet> {
        // create a new descriptor pool
        let new_pool = create_descriptor_pool(self.device.clone())?;
        self.descriptor_pools.push(new_pool.clone());

        // try allocation again
        new_pool
            .allocate_descriptor_set(set_layout)
            .context("allocating descriptor set for new egui texture")
    }
}

fn write_font_texture_desc_set(
    device: &Device,
    desc_set: &DescriptorSet,
    image_view: &ImageView<Image>,
    sampler: &Sampler,
) -> anyhow::Result<()> {
    let texture_info = vk::DescriptorImageInfo {
        image_view: image_view.handle(),
        image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        sampler: sampler.handle(),
    };

    let descriptor_writes = [vk::WriteDescriptorSet {
        dst_set: desc_set.handle(),
        descriptor_count: 1,
        descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
        p_image_info: &texture_info,
        ..Default::default()
    }];

    unsafe {
        device
            .inner()
            .update_descriptor_sets(&descriptor_writes, &[]);
    }

    Ok(())
}

fn create_descriptor_pool(device: Arc<Device>) -> anyhow::Result<Arc<DescriptorPool>> {
    let descriptor_pool_props = DescriptorPoolProperties {
        max_sets: MAX_DESC_SETS_PER_POOL,
        pool_sizes: vec![vk::DescriptorPoolSize {
            ty: vk::DescriptorType::SAMPLED_IMAGE,
            descriptor_count: MAX_DESC_SETS_PER_POOL,
        }],
        ..Default::default()
    };

    let descriptor_pool = DescriptorPool::new(device, descriptor_pool_props)
        .context("creating gui renderer descriptor pool")?;

    Ok(Arc::new(descriptor_pool))
}

fn create_texture_sampler(device: Arc<Device>) -> anyhow::Result<Arc<Sampler>> {
    let sampler_props = SamplerProperties {
        mag_filter: vk::Filter::LINEAR,
        min_filter: vk::Filter::LINEAR,
        address_mode: [vk::SamplerAddressMode::CLAMP_TO_EDGE; 3],
        mipmap_mode: vk::SamplerMipmapMode::LINEAR,
        ..Default::default()
    };

    let sampler = Sampler::new(device, sampler_props).context("creating gui texture sampler")?;
    Ok(Arc::new(sampler))
}

fn create_descriptor_layout(device: Arc<Device>) -> anyhow::Result<Arc<DescriptorSetLayout>> {
    let layout_props = DescriptorSetLayoutProperties::new(vec![DescriptorSetLayoutBinding {
        binding: descriptor::BINDING_FONT_TEXTURE,
        descriptor_type: vk::DescriptorType::SAMPLED_IMAGE,
        descriptor_count: 1,
        stage_flags: vk::ShaderStageFlags::FRAGMENT,
    }]);

    let desc_layout = DescriptorSetLayout::new(device, layout_props)
        .context("creating gui pass descriptor set layout")?;
    Ok(Arc::new(desc_layout))
}

fn create_pipeline_layout(
    device: Arc<Device>,
    desc_set_layout_texture: Arc<DescriptorSetLayout>,
) -> anyhow::Result<Arc<PipelineLayout>> {
    let mut pipeline_layout_props =
        PipelineLayoutProperties::new(vec![desc_set_layout_texture], Vec::new());

    let pipeline_layout = PipelineLayout::new(device, pipeline_layout_props)
        .context("creating gui pass pipeline layout")?;
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
            .context("creating gui pass vertex shader")?,
    );
    let vert_stage = ShaderStage::new(
        vk::ShaderStageFlags::VERTEX,
        vert_shader,
        CString::new(SHADER_ENTRY_POINT).context("shader entry point to c-string")?,
    );

    let frag_shader = Arc::new(
        ShaderModule::new_from_file(device.clone(), FRAG_SHADER_PATH)
            .context("creating gui pass fragment shader")?,
    );
    let frag_stage = ShaderStage::new(
        vk::ShaderStageFlags::FRAGMENT,
        frag_shader,
        CString::new(SHADER_ENTRY_POINT).context("shader entry point to c-string")?,
    );

    let shader_stages = [vert_stage, frag_stage];

    let dynamic_state =
        DynamicState::new_default(vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);
    let color_blend_state =
        ColorBlendState::new_default(vec![ColorBlendState::blend_state_alpha()]);

    let mut pipeline_properties = GraphicsPipelineProperties::default();
    pipeline_properties.subpass_index = subpass_index;
    pipeline_properties.dynamic_state = dynamic_state;
    pipeline_properties.color_blend_state = color_blend_state;
    pipeline_properties.vertex_input_state = EguiVertex::vertex_input_state();

    let pipeline = GraphicsPipeline::new(
        pipeline_layout,
        pipeline_properties,
        shader_stages,
        render_pass,
        None,
    )
    .context("creating gui pass pipeline")?;

    Ok(Arc::new(pipeline))
}

fn create_transient_command_pool(
    device: Arc<Device>,
    queue_family_index: u32,
) -> anyhow::Result<Arc<CommandPool>> {
    let command_pool_props = CommandPoolProperties {
        flags: vk::CommandPoolCreateFlags::TRANSIENT,
        queue_family_index,
    };

    let command_pool = CommandPool::new(device, command_pool_props)
        .context("creating gui renderer command pool")?;

    Ok(Arc::new(command_pool))
}

/// Caclulates the region of the framebuffer to render a gui element
fn calculate_gui_element_scissor(
    scale_factor: f32,
    framebuffer_dimensions: [f32; 2],
    rect: Rect,
) -> vk::Rect2D {
    let min = egui::Pos2 {
        x: rect.min.x * scale_factor,
        y: rect.min.y * scale_factor,
    };
    let min = egui::Pos2 {
        x: min.x.clamp(0.0, framebuffer_dimensions[0]),
        y: min.y.clamp(0.0, framebuffer_dimensions[1]),
    };
    let max = egui::Pos2 {
        x: rect.max.x * scale_factor,
        y: rect.max.y * scale_factor,
    };
    let max = egui::Pos2 {
        x: max.x.clamp(min.x, framebuffer_dimensions[0]),
        y: max.y.clamp(min.y, framebuffer_dimensions[1]),
    };
    vk::Rect2D {
        offset: vk::Offset2D {
            x: min.x.round() as i32,
            y: min.y.round() as i32,
        },
        extent: vk::Extent2D {
            width: (max.x.round() - min.x) as u32,
            height: (max.y.round() - min.y) as u32,
        },
    }
}

fn create_texture_data_buffer(
    memory_allocator: Arc<MemoryAllocator>,
    size: vk::DeviceSize,
) -> anyhow::Result<Buffer> {
    let buffer_props = BufferProperties {
        size,
        usage: vk::BufferUsageFlags::TRANSFER_SRC,
        ..Default::default()
    };

    let alloc_info = cpu_accessible_allocation_info();
    Buffer::new(memory_allocator, buffer_props, alloc_info).context("creating texture data buffer")
}

// ~~~ Errors ~~~

#[derive(Debug)]
pub enum GuiRendererError {
    /// Mesh requires a texture which doesn't exist (may have been prematurely destroyed or not yet created...)
    TextureDescSetMissing { id: TextureId },
}
impl std::error::Error for GuiRendererError {}
impl Display for GuiRendererError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::TextureDescSetMissing{id} =>
                write!(f, "Mesh requires texture [{:?}] which doesn't exist (may have been prematurely destroyed or not yet created...)", *id),
        }
    }
}
