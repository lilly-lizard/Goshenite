/// shout out to https://github.com/hakolao/egui_winit_vulkano for a lot of this code
use super::{
    common::{CreateDescriptorSetError, CreateShaderError},
    shaders::{push_constants::GuiPushConstants, vertex_inputs::EguiVertex},
};
use crate::{
    config::SHADER_ENTRY_POINT, renderer::common::create_shader_module, user_interface::gui::Gui,
};
use ahash::AHashMap;
use anyhow::Context;
use egui::{epaint::Primitive, ClippedPrimitive, Mesh, Rect, TextureId, TexturesDelta};
#[allow(unused_imports)]
use log::{debug, error, info, warn};
use smallvec::{smallvec, SmallVec};
use std::{
    fmt::{self, Display},
    sync::Arc,
};
use vulkano::{
    buffer::{
        cpu_access::CpuAccessibleBuffer, cpu_pool::CpuBufferPoolChunk, BufferUsage, CpuBufferPool,
        TypedBufferAccess,
    },
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder, BufferImageCopy,
        CommandBufferUsage, CopyBufferToImageInfo,
    },
    descriptor_set::{
        allocator::StandardDescriptorSetAllocator, layout::DescriptorSetLayout,
        PersistentDescriptorSet, WriteDescriptorSet,
    },
    device::{Device, Queue},
    format::Format,
    image::{
        view::ImageView, ImageAccess, ImageLayout, ImageUsage, ImageViewAbstract, ImmutableImage,
    },
    memory::allocator::{MemoryAllocator, MemoryUsage, StandardMemoryAllocator},
    pipeline::{
        graphics::{
            color_blend::{AttachmentBlend, BlendFactor, ColorBlendState},
            input_assembly::InputAssemblyState,
            rasterization::{CullMode, RasterizationState},
            vertex_input::BuffersDefinition,
            viewport::{Scissor, Viewport, ViewportState},
            GraphicsPipeline,
        },
        Pipeline, PipelineBindPoint,
    },
    render_pass::Subpass,
    sampler::{self, Sampler, SamplerAddressMode, SamplerCreateInfo, SamplerMipmapMode},
    sync::GpuFuture,
    DeviceSize,
};

const VERT_SHADER_PATH: &str = "assets/shader_binaries/gui.vert.spv";
const FRAG_SHADER_PATH: &str = "assets/shader_binaries/gui.frag.spv";

const VERTICES_PER_QUAD: DeviceSize = 4;
const VERTEX_BUFFER_SIZE: DeviceSize = 1024 * 1024 * VERTICES_PER_QUAD;
const INDEX_BUFFER_SIZE: DeviceSize = 1024 * 1024 * 2;

const TEXTURE_FORMAT: Format = Format::R8G8B8A8_SRGB;

mod descriptor {
    pub const SET_FONT_TEXTURE: usize = 0;
    pub const BINDING_FONT_TEXTURE: u32 = 0;
}

/// Index format
type VertexIndex = u32;

pub struct GuiRenderer {
    device: Arc<Device>,
    memory_allocator: Arc<dyn MemoryAllocator>,
    transfer_queue: Arc<Queue>,

    pipeline: Arc<GraphicsPipeline>,
    sampler: Arc<Sampler>,
    vertex_buffer_pool: CpuBufferPool<EguiVertex>,
    index_buffer_pool: CpuBufferPool<VertexIndex>,

    texture_images: AHashMap<egui::TextureId, Arc<dyn ImageViewAbstract>>,
    texture_desc_sets: AHashMap<egui::TextureId, Arc<PersistentDescriptorSet>>,
}
// Public functions
impl GuiRenderer {
    /// Initializes the gui renderer
    pub fn new(
        device: Arc<Device>,
        memory_allocator: Arc<StandardMemoryAllocator>,
        transfer_queue: Arc<Queue>,
        subpass: Subpass,
    ) -> anyhow::Result<Self> {
        let pipeline = create_pipeline(device.clone(), subpass)?;
        let (vertex_buffer_pool, index_buffer_pool) =
            create_buffer_pools(device.clone(), memory_allocator)?;
        let sampler = Self::create_sampler(device.clone())?;
        Ok(Self {
            device,
            memory_allocator,
            transfer_queue,
            pipeline,
            sampler,
            vertex_buffer_pool,
            index_buffer_pool,
            texture_images: AHashMap::default(),
            texture_desc_sets: AHashMap::default(),
        })
    }

    /// Creates and/or removes texture resources as required by [`TexturesDelta`](epaint::Textures::TexturesDelta)
    /// output by [`egui::end_frame`](egui::context::Context::end_frame).
    pub fn update_textures(
        &mut self,
        exec_after_future: Box<dyn GpuFuture>,
        command_buffer_allocator: &StandardCommandBufferAllocator,
        descriptor_allocator: &StandardDescriptorSetAllocator,
        textures_delta_vec: Vec<TexturesDelta>,
        render_queue: Arc<Queue>,
    ) -> anyhow::Result<Box<dyn GpuFuture>> {
        // return if empty
        if textures_delta_vec.is_empty() {
            return Ok(exec_after_future);
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
        let finished = exec_after_future
            .then_execute(self.transfer_queue.clone(), command_buffer)
            .context("executing gui texture upload commands")?;
        let future = finished
            .then_signal_fence_and_flush()
            .context("executing gui texture upload commands")?;
        Ok(future.boxed())
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

        let push_constants = GuiPushConstants::new(
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
                    let scissors = [get_rect_scissor(
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
    /// Create sampler for gui textures.
    fn create_sampler(device: Arc<Device>) -> anyhow::Result<Arc<Sampler>> {
        Sampler::new(
            device,
            SamplerCreateInfo {
                mag_filter: sampler::Filter::Linear,
                min_filter: sampler::Filter::Linear,
                address_mode: [SamplerAddressMode::ClampToEdge; 3],
                mipmap_mode: SamplerMipmapMode::Linear,
                ..Default::default()
            },
        )
        .context("creating gui texture sampler")
    }

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
            egui::ImageData::Font(image) => {
                let gamma = 1.0;
                image
                    .srgba_pixels(gamma)
                    .flat_map(|color| color.to_array())
                    .collect()
            }
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
        image: Arc<impl ImageViewAbstract>,
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

/// Builds the gui rendering graphics pipeline.
///
/// Helper function for [`Self::new`]
fn create_pipeline(device: Arc<Device>, subpass: Subpass) -> anyhow::Result<Arc<GraphicsPipeline>> {
    let mut blend = AttachmentBlend::alpha();
    blend.color_source = BlendFactor::One;
    let blend_state = ColorBlendState::new(1).blend(blend);
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
    GraphicsPipeline::start()
        .vertex_input_state(BuffersDefinition::new().vertex::<EguiVertex>())
        .vertex_shader(vert_shader, ())
        .input_assembly_state(InputAssemblyState::new())
        .fragment_shader(frag_shader, ())
        .viewport_state(ViewportState::viewport_dynamic_scissor_dynamic(1))
        .color_blend_state(blend_state)
        .rasterization_state(RasterizationState::new().cull_mode(CullMode::None))
        .render_pass(subpass)
        .build(device.clone())
        .context("gui pipeline")
}

/// Creates vertex and index buffer pools.
///
/// Helper function for [`Self::new`]
fn create_buffer_pools(
    device: Arc<Device>,
    memory_allocator: Arc<StandardMemoryAllocator>,
) -> anyhow::Result<(CpuBufferPool<EguiVertex>, CpuBufferPool<VertexIndex>)> {
    let vertex_buffer_pool = CpuBufferPool::vertex_buffer(memory_allocator);
    vertex_buffer_pool
        .reserve(VERTEX_BUFFER_SIZE)
        .context("creating gui vertex buffer pool")?;
    debug!(
        "reserving {} bytes for gui vertex buffer pool",
        VERTEX_BUFFER_SIZE
    );

    let index_buffer_pool = CpuBufferPool::new(
        memory_allocator,
        BufferUsage {
            index_buffer: true,
            ..BufferUsage::empty()
        },
        MemoryUsage::Upload,
    );
    index_buffer_pool
        .reserve(INDEX_BUFFER_SIZE)
        .context("creating gui index buffer pool")?;
    debug!(
        "reserving {} bytes for gui index buffer pool",
        INDEX_BUFFER_SIZE
    );

    Ok((vertex_buffer_pool, index_buffer_pool))
}

/// Caclulates the region of the framebuffer to render a gui element.
fn get_rect_scissor(scale_factor: f32, framebuffer_dimensions: [f32; 2], rect: Rect) -> Scissor {
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
    Scissor {
        origin: [min.x.round() as u32, min.y.round() as u32],
        dimensions: [
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
