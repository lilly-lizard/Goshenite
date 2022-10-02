/// shout out to https://github.com/hakolao/egui_winit_vulkano for a lot of this code
use super::common::{CreateDescriptorSetError, CreateShaderError};
use crate::gui::Gui;
use crate::renderer::common::create_shader_module;
use crate::shaders::shader_interfaces::{self, SHADER_ENTRY_POINT};
use ahash::AHashMap;
use anyhow::Context;
use egui::{epaint::Primitive, ClippedPrimitive, Mesh, Rect, TextureId};
#[allow(unused_imports)]
use log::{debug, error, info, warn};
use std::fmt::{self, Display};
use std::sync::Arc;
use vulkano::{
    buffer::{
        cpu_access::CpuAccessibleBuffer, cpu_pool::CpuBufferPoolChunk, BufferUsage, CpuBufferPool,
        TypedBufferAccess,
    },
    command_buffer::{
        AutoCommandBufferBuilder, BufferImageCopy, CommandBufferUsage, CopyBufferToImageInfo,
        PrimaryCommandBuffer,
    },
    descriptor_set::{layout::DescriptorSetLayout, PersistentDescriptorSet, WriteDescriptorSet},
    device::{Device, Queue},
    format::Format,
    image::{
        view::ImageView, ImageAccess, ImageLayout, ImageUsage, ImageViewAbstract, ImmutableImage,
    },
    memory::pool::StandardMemoryPool,
    pipeline::{
        graphics::{
            color_blend::{AttachmentBlend, BlendFactor, ColorBlendState},
            input_assembly::InputAssemblyState,
            rasterization::{CullMode, RasterizationState},
            render_pass::PipelineRenderingCreateInfo,
            vertex_input::BuffersDefinition,
            viewport::{Scissor, Viewport, ViewportState},
            GraphicsPipeline,
        },
        Pipeline, PipelineBindPoint,
    },
    sampler::{self, Sampler, SamplerAddressMode, SamplerCreateInfo, SamplerMipmapMode},
    sync::GpuFuture,
    DeviceSize,
};

const VERT_SHADER_PATH: &str = "assets/shader_binaries/gui.vert.spv";
const FRAG_SHADER_PATH: &str = "assets/shader_binaries/gui.frag.spv";

const VERTICES_PER_QUAD: DeviceSize = 4;
const VERTEX_BUFFER_SIZE: DeviceSize = 1024 * 1024 * VERTICES_PER_QUAD;
const INDEX_BUFFER_SIZE: DeviceSize = 1024 * 1024 * 2;

mod descriptor {
    pub const SET_FONT_TEXTURE: usize = 0;
    pub const BINDING_FONT_TEXTURE: u32 = 0;
}

/// Should match vertex definition of egui (except color is `[f32; 4]`)
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
struct EguiVertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
    pub color: [f32; 4],
}
vulkano::impl_vertex!(EguiVertex, position, tex_coords, color);

/// Index format
type VertexIndex = u32;

pub struct GuiRenderer {
    device: Arc<Device>,
    transfer_queue: Arc<Queue>,

    pipeline: Arc<GraphicsPipeline>,
    sampler: Arc<Sampler>,
    vertex_buffer_pool: CpuBufferPool<EguiVertex>,
    index_buffer_pool: CpuBufferPool<VertexIndex>,

    texture_images: AHashMap<egui::TextureId, Arc<dyn ImageViewAbstract + Send + Sync + 'static>>,
    texture_desc_sets: AHashMap<egui::TextureId, Arc<PersistentDescriptorSet>>,
}
// Public functions
impl GuiRenderer {
    /// Initializes the gui renderer
    pub(super) fn new(
        device: Arc<Device>,
        transfer_queue: Arc<Queue>,
        swapchain_image_format: Format,
    ) -> anyhow::Result<Self> {
        let pipeline = create_pipeline(device.clone(), swapchain_image_format)?;
        let (vertex_buffer_pool, index_buffer_pool) = create_buffer_pools(device.clone())?;
        let sampler = Self::create_sampler(device.clone())?;
        Ok(Self {
            device,
            transfer_queue,
            pipeline,
            sampler,
            vertex_buffer_pool,
            index_buffer_pool,
            texture_images: AHashMap::default(),
            texture_desc_sets: AHashMap::default(),
        })
    }

    /// Creates and/or removes texture resources for a [`Gui`](crate::gui::Gui) frame.
    pub fn update_textures(&mut self, textures_delta: egui::TexturesDelta) -> anyhow::Result<()> {
        // release unused texture resources
        for &id in &textures_delta.free {
            self.unregister_image(id);
        }

        // create command buffer builder
        let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
            self.device.clone(),
            self.transfer_queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .context("creating command buffer for gui texture upload")?;

        // create new images and record upload commands
        for (id, image_delta) in textures_delta.set {
            self.create_texture(id, image_delta, &mut command_buffer_builder)?;
        }

        // execute command buffer
        let command_buffer = command_buffer_builder
            .build()
            .context("building command buffer for gui texture upload")?;
        let finished = command_buffer
            .execute(self.transfer_queue.clone())
            .context("executing gui texture upload commands")?;
        // todo flush blocks thread... pass onto renderer manager
        let _future = finished
            .then_signal_fence_and_flush()
            .context("executing gui texture upload commands")?;
        Ok(())
    }

    /// Record gui rendering commands
    /// * `command_buffer`: Primary command buffer to record commands to. Must be already in dynamic rendering state.
    /// * `primitives`: List of egui primitives to render. Can aquire from [Gui::primitives](`crate::gui::Gui::primitives`).
    /// * `scale_factor`: Gui dpi config. Can aquire from [Gui::scale_factor](`crate::gui::Gui::scale_factor`).
    /// * `need_srgb_conv`: Set to true if rendering to an SRGB framebuffer.
    /// * `framebuffer_dimensions`: Framebuffer dimensions.
    pub(super) fn record_commands<L>(
        &mut self,
        command_buffer: &mut AutoCommandBufferBuilder<L>,
        gui: &Gui,
        need_srgb_conv: bool,
        framebuffer_dimensions: [u32; 2],
    ) -> anyhow::Result<()> {
        let scale_factor = gui.scale_factor();
        let primitives = gui.primitives();

        let push_constants = shader_interfaces::GuiPushConstant::new(
            [
                framebuffer_dimensions[0] as f32 / scale_factor,
                framebuffer_dimensions[1] as f32 / scale_factor,
            ],
            need_srgb_conv,
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
                    let (vertices, indices) = self.create_subbuffers(mesh)?;

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
                                dimensions: [
                                    framebuffer_dimensions[0] as f32,
                                    framebuffer_dimensions[1] as f32,
                                ],
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
                        .context("recording gui render commands")?;
                }
                _ => continue, // don't need to support Primitive::Callback
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
        texture_id: egui::TextureId,
        delta: egui::epaint::ImageDelta,
        command_buffer_builder: &mut AutoCommandBufferBuilder<L>,
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
            self.device.clone(),
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

            // create image
            let (image, init_access) = ImmutableImage::uninitialized(
                self.device.clone(),
                vulkano::image::ImageDimensions::Dim2d {
                    width: delta.image.width() as u32,
                    height: delta.image.height() as u32,
                    array_layers: 1,
                },
                Format::R8G8B8A8_SRGB,
                vulkano::image::MipmapsCount::One,
                ImageUsage {
                    transfer_dst: true,
                    //transfer_src: true, // todo needed? try without
                    sampled: true,
                    ..ImageUsage::empty()
                },
                Default::default(),
                ImageLayout::ShaderReadOnlyOptimal,
                None,
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
                })
                .context("creating new gui texture desc set")?;
            let font_desc_set = self
                .sampled_image_desc_set(layout, font_image.clone())
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
        self.texture_desc_sets.remove(&texture_id);
        self.texture_images.remove(&texture_id);
    }

    /// Create vertex and index sub-buffers for an egui mesh
    fn create_subbuffers(
        &self,
        mesh: &Mesh,
    ) -> anyhow::Result<(
        Arc<CpuBufferPoolChunk<EguiVertex, Arc<StandardMemoryPool>>>,
        Arc<CpuBufferPoolChunk<VertexIndex, Arc<StandardMemoryPool>>>,
    )> {
        // copy vertices to buffer
        let v_slice = &mesh.vertices;

        let vertex_chunk = self
            .vertex_buffer_pool
            .from_iter(v_slice.into_iter().map(|v| EguiVertex {
                position: [v.pos.x, v.pos.y],
                tex_coords: [v.uv.x, v.uv.y],
                color: [
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
        layout: &Arc<DescriptorSetLayout>,
        image: Arc<dyn ImageViewAbstract + 'static>,
    ) -> anyhow::Result<Arc<PersistentDescriptorSet>> {
        PersistentDescriptorSet::new(
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
fn create_pipeline(
    device: Arc<Device>,
    swapchain_image_format: Format,
) -> anyhow::Result<Arc<GraphicsPipeline>> {
    let mut blend = AttachmentBlend::alpha();
    blend.color_source = BlendFactor::One;
    let blend_state = ColorBlendState::new(1).blend(blend);
    let vert_module = create_shader_module(device.clone(), VERT_SHADER_PATH)?;
    let vert_shader =
        vert_module
            .entry_point(SHADER_ENTRY_POINT)
            .ok_or(CreateShaderError::MissingEntryPoint(
                VERT_SHADER_PATH.to_string(),
            ))?;
    let frag_module = create_shader_module(device.clone(), FRAG_SHADER_PATH)?;
    let frag_shader =
        frag_module
            .entry_point(SHADER_ENTRY_POINT)
            .ok_or(CreateShaderError::MissingEntryPoint(
                FRAG_SHADER_PATH.to_string(),
            ))?;
    Ok(GraphicsPipeline::start()
        .vertex_input_state(BuffersDefinition::new().vertex::<EguiVertex>())
        .vertex_shader(vert_shader, ())
        .input_assembly_state(InputAssemblyState::new())
        .fragment_shader(frag_shader, ())
        .viewport_state(ViewportState::viewport_dynamic_scissor_dynamic(1))
        .color_blend_state(blend_state)
        .rasterization_state(RasterizationState::new().cull_mode(CullMode::None))
        .render_pass(PipelineRenderingCreateInfo {
            color_attachment_formats: vec![Some(swapchain_image_format)],
            ..Default::default()
        })
        .build(device.clone())
        .context("gui pipeline")?)
}

/// Creates vertex and index buffer pools.
///
/// Helper function for [`Self::new`]
fn create_buffer_pools(
    device: Arc<Device>,
) -> anyhow::Result<(CpuBufferPool<EguiVertex>, CpuBufferPool<VertexIndex>)> {
    let vertex_buffer_pool = CpuBufferPool::vertex_buffer(device.clone());
    vertex_buffer_pool
        .reserve(VERTEX_BUFFER_SIZE)
        .context("creating gui vertex buffer pool")?;
    debug!(
        "reserving {} bytes for gui vertex buffer pool",
        VERTEX_BUFFER_SIZE
    );

    let index_buffer_pool = CpuBufferPool::new(
        device,
        BufferUsage {
            index_buffer: true,
            ..BufferUsage::empty()
        },
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
fn get_rect_scissor(scale_factor: f32, framebuffer_dimensions: [u32; 2], rect: Rect) -> Scissor {
    let min = egui::Pos2 {
        x: rect.min.x * scale_factor,
        y: rect.min.y * scale_factor,
    };
    let min = egui::Pos2 {
        x: min.x.clamp(0.0, framebuffer_dimensions[0] as f32),
        y: min.y.clamp(0.0, framebuffer_dimensions[1] as f32),
    };
    let max = egui::Pos2 {
        x: rect.max.x * scale_factor,
        y: rect.max.y * scale_factor,
    };
    let max = egui::Pos2 {
        x: max.x.clamp(min.x, framebuffer_dimensions[0] as f32),
        y: max.y.clamp(min.y, framebuffer_dimensions[1] as f32),
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
