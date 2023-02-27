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
    descriptor_layout::{
        DescriptorSetLayout, DescriptorSetLayoutBinding, DescriptorSetLayoutProperties,
    },
    descriptor_set::DescriptorSet,
    device::Device,
    image::Image,
    image_view::ImageView,
    memory::MemoryAllocator,
    pipeline_graphics::{
        ColorBlendState, DynamicState, GraphicsPipeline, GraphicsPipelineProperties,
    },
    pipeline_layout::{PipelineLayout, PipelineLayoutProperties},
    queue::Queue,
    render_pass::RenderPass,
    sampler::{Sampler, SamplerProperties},
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

const VERT_SHADER_PATH: &str = "assets/shader_binaries/gui.vert.spv";
const FRAG_SHADER_PATH: &str = "assets/shader_binaries/gui.frag.spv";

const VERTICES_PER_QUAD: vk::DeviceSize = 4;
const VERTEX_BUFFER_SIZE: vk::DeviceSize = 1024 * 1024 * VERTICES_PER_QUAD;
const INDEX_BUFFER_SIZE: vk::DeviceSize = 1024 * 1024 * 2;

const TEXTURE_FORMAT: vk::Format = vk::Format::R8G8B8A8_SRGB;

mod descriptor {
    pub const SET_FONT_TEXTURE: usize = 0;
    pub const BINDING_FONT_TEXTURE: u32 = 0;
}

/// Index format
type VertexIndex = u32;

pub struct GuiRenderer {
    memory_allocator: Arc<MemoryAllocator>,
    transfer_queue: Arc<Queue>,

    pipeline: Arc<GraphicsPipeline>,
    texture_sampler: Arc<Sampler>,

    texture_images: AHashMap<egui::TextureId, Arc<ImageView<Image>>>,
    texture_desc_sets: AHashMap<egui::TextureId, Arc<DescriptorSet>>,
}
// Public functions
impl GuiRenderer {
    /// Initializes the gui renderer
    pub fn new(
        device: Arc<Device>,
        memory_allocator: Arc<MemoryAllocator>,
        transfer_queue: Arc<Queue>,
        render_pass: &RenderPass,
        subpass_index: u32,
    ) -> anyhow::Result<Self> {
        let desc_set_layout = create_descriptor_layout(device.clone())?;

        let pipeline_layout = create_pipeline_layout(device.clone(), desc_set_layout)?;
        let pipeline =
            create_pipeline(device.clone(), pipeline_layout, render_pass, subpass_index)?;

        let texture_sampler = create_texture_sampler(device.clone())?;

        Ok(Self {
            memory_allocator,
            transfer_queue,
            pipeline,
            texture_sampler,
            texture_images: AHashMap::default(),
            texture_desc_sets: AHashMap::default(),
        })
    }

    /// Creates and/or removes texture resources as required by [`TexturesDelta`](epaint::Textures::TexturesDelta)
    /// output by [`egui::end_frame`](egui::context::Context::end_frame).
    pub fn update_textures(
        &mut self,
        command_buffer_allocator: &StandardCommandBufferAllocator,
        textures_delta_vec: Vec<TexturesDelta>,
        render_queue: Arc<Queue>,
    ) -> anyhow::Result<()> {
        // return if empty
        if textures_delta_vec.is_empty() {
            return Ok(());
        }

        // create command buffer builder
        let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
            command_buffer_allocator,
            self.transfer_queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .context("creating command buffer for gui texture upload")?;

        for textures_delta in textures_delta_vec {
            // release unused texture resources
            for &id in &textures_delta.free {
                self.unregister_image(id);
            }

            // create new images and record upload commands
            for (id, image_delta) in textures_delta.set {
                self.create_texture(
                    descriptor_allocator,
                    id,
                    image_delta,
                    &mut command_buffer_builder,
                    render_queue.clone(),
                )?;
            }
        }

        // execute command buffer
        let command_buffer = command_buffer_builder
            .build()
            .context("building command buffer for gui texture upload")?;
        todo!("return semaphore or something?");

        Ok(())
    }

    /// Record gui rendering commands
    /// * `command_buffer`: Primary command buffer to record commands to. Must be already in dynamic rendering state.
    /// * `primitives`: List of egui primitives to render. Can aquire from [Gui::primitives](`crate::gui::Gui::primitives`).
    /// * `scale_factor`: Gui dpi config. Can aquire from [Gui::scale_factor](`crate::gui::Gui::scale_factor`).
    /// * `is_srgb_framebuffer`: Set to true if rendering to an SRGB framebuffer.
    /// * `framebuffer_dimensions`: Framebuffer dimensions.
    pub(super) fn record_commands<L>(
        &mut self,
        command_buffer: &mut AutoCommandBufferBuilder<L>,
        gui: &Gui,
        is_srgb_framebuffer: bool,
        framebuffer_dimensions: [f32; 2],
    ) -> anyhow::Result<()> {
        let scale_factor = gui.scale_factor();
        let primitives = gui.mesh_primitives();

        let push_constants = GuiPushConstant::new(
            [
                framebuffer_dimensions[0] / scale_factor,
                framebuffer_dimensions[1] / scale_factor,
            ],
            is_srgb_framebuffer,
        );
        for ClippedPrimitive {
            clip_rect,
            primitive,
        } in primitives
        {
            match primitive {
                Primitive::Mesh(mesh) => {
                    // nothing to draw if we don't have vertices & indices
                    if mesh.vertices.is_empty() || mesh.indices.is_empty() {
                        continue;
                    }

                    // get region of screen to render
                    let scissors = [calculate_gui_element_scissor(
                        scale_factor,
                        framebuffer_dimensions,
                        *clip_rect,
                    )];

                    // create vertex and index buffers
                    let (vertices, indices) = self.create_subbuffers(&mesh)?;

                    let desc_set = self
                        .texture_desc_sets
                        .get(&mesh.texture_id)
                        .ok_or(GuiRendererError::TextureDescSetMissing {
                            id: mesh.texture_id,
                        })
                        .context("recording gui render commands")?
                        .clone();

                    command_buffer
                        .bind_pipeline_graphics(self.pipeline.clone())
                        .set_viewport(
                            0,
                            [Viewport {
                                origin: [0.0, 0.0],
                                dimensions: framebuffer_dimensions,
                                depth_range: 0.0..1.0,
                            }],
                        )
                        .set_scissor(0, scissors)
                        .bind_descriptor_sets(
                            PipelineBindPoint::Graphics,
                            self.pipeline.layout().clone(),
                            0,
                            desc_set.clone(),
                        )
                        .push_constants(self.pipeline.layout().clone(), 0, push_constants)
                        .bind_vertex_buffers(0, vertices.clone())
                        .bind_index_buffer(indices.clone())
                        .draw_indexed(indices.len() as u32, 1, 0, 0, 0)
                        .context("recording gui draw commands")?;
                }
                Primitive::Callback(_) => continue, // we don't need to support Primitive::Callback
            }
        }
        Ok(())
    }
}

// Private functions

impl GuiRenderer {
    /// Creates a new texture needing to be added for the gui.
    ///
    /// Helper function for [`GuiRenderer::update_textures`]
    fn create_texture<L>(
        &mut self,
        descriptor_allocator: &StandardDescriptorSetAllocator,
        texture_id: egui::TextureId,
        delta: egui::epaint::ImageDelta,
        command_buffer_builder: &mut AutoCommandBufferBuilder<L>,
        render_queue: Arc<Queue>,
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
        let texture_data_buffer = CpuAccessibleBuffer::from_iter(
            self.memory_allocator.as_ref(),
            BufferUsage {
                transfer_src: true,
                ..BufferUsage::empty()
            },
            false,
            data,
        )
        .context("creating gui texture data buffer")?;

        if let Some(update_pos) = delta.pos {
            // sometimes a subregion of an already allocated texture needs to be updated e.g. when a font size is changed
            // todo sync issue!
            // CommandBufferExecError(AccessError { error: AlreadyInUse, command_name: "copy_buffer_to_image", command_param: "dst_image", command_offset: 0 })
            // pass future to update_textures and this funtion sets a bool to indicate wherver an existing will be modified...
            if let Some(existing_image) = self.texture_images.get(&texture_id) {
                // define copy region
                let copy_region = BufferImageCopy {
                    image_subresource: existing_image.image().subresource_layers(),
                    image_offset: [update_pos[0] as u32, update_pos[1] as u32, 0],
                    image_extent: [delta.image.width() as u32, delta.image.height() as u32, 1],
                    ..Default::default()
                };
                debug!("updating existing gui texture id = {:?}, region offset = {:?}, region extent = {:?}",
                    texture_id, copy_region.image_offset, copy_region.image_extent);

                // copy buffer to image
                command_buffer_builder
                    .copy_buffer_to_image(CopyBufferToImageInfo {
                        regions: [copy_region].into(),
                        ..CopyBufferToImageInfo::buffer_image(
                            texture_data_buffer,
                            existing_image.image().clone(),
                        )
                    })
                    .context("updating region of existing gui texture")?;
            }
        } else {
            // usually ImageDelta.pos == None meaning a new image needs to be created
            debug!("creating new gui texture id = {:?}", texture_id);

            // create image
            let transfer_queue_family = self.transfer_queue.queue_family_index();
            let render_queue_family = render_queue.queue_family_index();
            let queue_family_indices: SmallVec<[u32; 2]> =
                if transfer_queue_family == render_queue_family {
                    // will result in VK_SHARING_MODE_EXCLUSIVE
                    smallvec![render_queue_family]
                } else {
                    // will result in VK_SHARING_MODE_CONCURRENT
                    smallvec![render_queue_family, transfer_queue_family]
                };
            let (image, init_access) = ImmutableImage::uninitialized(
                self.memory_allocator.as_ref(),
                vulkano::image::ImageDimensions::Dim2d {
                    width: delta.image.width() as u32,
                    height: delta.image.height() as u32,
                    array_layers: 1,
                },
                TEXTURE_FORMAT,
                vulkano::image::MipmapsCount::One,
                ImageUsage {
                    transfer_dst: true,
                    sampled: true,
                    ..ImageUsage::empty()
                },
                Default::default(),
                ImageLayout::ShaderReadOnlyOptimal,
                queue_family_indices,
            )
            .context("creating new gui texture image")?;
            let font_image =
                ImageView::new_default(image).context("creating new gui texture image")?;

            // copy buffer to image
            command_buffer_builder
                .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
                    texture_data_buffer,
                    init_access.clone(),
                ))
                .context("uploading new gui texture data")?;

            // create new descriptor set
            let layout = self
                .pipeline
                .layout()
                .set_layouts()
                .get(descriptor::SET_FONT_TEXTURE)
                .ok_or(CreateDescriptorSetError::InvalidDescriptorSetIndex {
                    index: descriptor::SET_FONT_TEXTURE,
                    shader_path: FRAG_SHADER_PATH,
                })
                .context("creating new gui texture desc set")?;
            let font_desc_set = self
                .sampled_image_desc_set(descriptor_allocator, layout, font_image.clone())
                .context("creating new gui texture desc set")?;

            // store new texture
            self.texture_desc_sets.insert(texture_id, font_desc_set);
            self.texture_images.insert(texture_id, font_image);
        }
        Ok(())
    }

    /// Unregister a texture that is no longer required by the gui.
    ///
    /// Helper function for [`Self::update_textures`]
    fn unregister_image(&mut self, texture_id: egui::TextureId) {
        debug!("removing unneeded gui texture id = {:?}", texture_id);
        self.texture_desc_sets.remove(&texture_id);
        self.texture_images.remove(&texture_id);
    }

    /// Create vertex and index sub-buffers for an egui mesh
    fn create_subbuffers(
        &self,
        mesh: &Mesh,
    ) -> anyhow::Result<(
        Arc<CpuBufferPoolChunk<EguiVertex>>,
        Arc<CpuBufferPoolChunk<VertexIndex>>,
    )> {
        // copy vertices to buffer
        let v_slice = &mesh.vertices;

        let vertex_chunk = self
            .vertex_buffer_pool
            .from_iter(v_slice.into_iter().map(|v| EguiVertex {
                in_position: v.pos.into(),
                in_tex_coords: v.uv.into(),
                in_color: [
                    v.color.r() as f32 / 255.0,
                    v.color.g() as f32 / 255.0,
                    v.color.b() as f32 / 255.0,
                    v.color.a() as f32 / 255.0,
                ],
            }))
            .context("creating gui vertex subbuffer")?;

        // Copy indices to buffer
        let i_slice = &mesh.indices;
        let index_chunk = self
            .index_buffer_pool
            .from_iter(i_slice.clone())
            .context("creating gui index subbuffer")?;

        Ok((vertex_chunk, index_chunk))
    }

    /// Creates a descriptor set for images
    fn sampled_image_desc_set(
        &self,
        descriptor_allocator: &StandardDescriptorSetAllocator,
        layout: &Arc<DescriptorSetLayout>,
        image: Arc<impl ImageViewAbstract + 'static>,
    ) -> anyhow::Result<Arc<PersistentDescriptorSet>> {
        PersistentDescriptorSet::new(
            descriptor_allocator,
            layout.clone(),
            [WriteDescriptorSet::image_view_sampler(
                descriptor::BINDING_FONT_TEXTURE,
                image.clone(),
                self.sampler.clone(),
            )],
        )
        .context("creating gui texture descriptor set")
    }
}

fn create_texture_sampler(device: Arc<Device>) -> anyhow::Result<Arc<Sampler>> {
    let sampler_props = SamplerProperties {
        mag_filter: vk::Filter::Linear,
        min_filter: vk::Filter::Linear,
        address_mode: [vk::SamplerAddressMode::ClampToEdge; 3],
        mipmap_mode: vk::SamplerMipmapMode::Linear,
        ..Default::default()
    };

    let sampler = Sampler::new(device, sampler_props).context("creating gui texture sampler");
    Arc::new(sampler)
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
        offset: [min.x.round() as i32, min.y.round() as i32],
        extent: [
            (max.x.round() - min.x) as u32,
            (max.y.round() - min.y) as u32,
        ],
    }
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
