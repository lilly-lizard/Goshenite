/// shout out to https://github.com/hakolao/egui_winit_vulkano for a lot of this code
use crate::renderer::render_manager::{create_shader_module, RenderManagerError};
use crate::shaders::shader_interfaces;
use ahash::AHashMap;
use egui::{epaint::Primitive, ClippedPrimitive, Mesh, Rect};
#[allow(unused_imports)]
use log::{debug, error, info, warn};
use std::sync::Arc;
use vulkano::{
    buffer::{
        cpu_access::CpuAccessibleBuffer, cpu_pool::CpuBufferPoolChunk, BufferUsage, CpuBufferPool,
        TypedBufferAccess,
    },
    command_buffer::{
        AutoCommandBufferBuilder, BlitImageInfo, CommandBufferUsage, CopyBufferToImageInfo,
        ImageBlit, PrimaryAutoCommandBuffer, PrimaryCommandBuffer,
    },
    descriptor_set::{layout::DescriptorSetLayout, PersistentDescriptorSet, WriteDescriptorSet},
    device::{Device, Queue},
    format::Format,
    image::{
        view::ImageView, ImageAccess, ImageLayout, ImageUsage, ImageViewAbstract, ImmutableImage,
    },
    memory::pool::StdMemoryPool,
    pipeline::{
        graphics::{
            color_blend::{AttachmentBlend, BlendFactor, ColorBlendState},
            input_assembly::InputAssemblyState,
            rasterization::{CullMode, RasterizationState},
            render_pass::PipelineRenderingCreateInfo,
            viewport::{Scissor, Viewport, ViewportState},
        },
        graphics::{vertex_input::BuffersDefinition, GraphicsPipeline},
        Pipeline, PipelineBindPoint,
    },
    sampler::{self, Sampler, SamplerAddressMode, SamplerCreateInfo, SamplerMipmapMode},
    sync::GpuFuture,
    DeviceSize,
};

const VERTICES_PER_QUAD: DeviceSize = 4;
const VERTEX_BUFFER_SIZE: DeviceSize = 1024 * 1024 * VERTICES_PER_QUAD;
const INDEX_BUFFER_SIZE: DeviceSize = 1024 * 1024 * 2;

/// Should match vertex definition of egui (except color is `[f32; 4]`)
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct EguiVertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
    pub color: [f32; 4],
}
vulkano::impl_vertex!(EguiVertex, position, tex_coords, color);

pub struct GuiRenderer {
    device: Arc<Device>,
    queue: Arc<Queue>,

    pipeline: Arc<GraphicsPipeline>,
    sampler: Arc<Sampler>,
    vertex_buffer_pool: CpuBufferPool<EguiVertex>,
    index_buffer_pool: CpuBufferPool<u32>,

    texture_images: AHashMap<egui::TextureId, Arc<dyn ImageViewAbstract + Send + Sync + 'static>>,
    texture_desc_sets: AHashMap<egui::TextureId, Arc<PersistentDescriptorSet>>,
}
// Public functions
impl GuiRenderer {
    /// Initializes the gui renderer
    pub fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        swapchain_image_format: Format,
    ) -> Result<Self, RenderManagerError> {
        let pipeline = Self::create_pipeline(device.clone(), swapchain_image_format)?;
        let sampler = Self::create_sampler(device.clone());
        let (vertex_buffer_pool, index_buffer_pool) = Self::create_buffer_pools(device.clone());

        Ok(Self {
            device: device.clone(),
            queue: queue.clone(),
            pipeline,
            sampler,
            vertex_buffer_pool,
            index_buffer_pool,
            texture_images: AHashMap::default(),
            texture_desc_sets: AHashMap::default(),
        })
    }

    /// Creates and/or removes texture resources for a [Gui](`crate::gui::Gui) frame.
    pub fn update_textures(&mut self, textures_delta: &egui::TexturesDelta) {
        for &id in &textures_delta.free {
            self.unregister_image(id);
        }
        for (id, image_delta) in &textures_delta.set {
            self.create_texture(*id, image_delta);
        }
    }

    /// Record gui rendering commands
    /// * `command_buffer`: Primary command buffer to record commands to. Must be already in dynamic rendering state.
    /// * `primitives`: List of egui primitives to render. Can aquire from [Gui::primitives](`crate::gui::Gui::primitives`).
    /// * `scale_factor`: Gui dpi config. Can aquire from [Gui::scale_factor](`crate::gui::Gui::scale_factor`).
    /// * `need_srgb_conv`: Set to true if rendering to an SRGB framebuffer.
    /// * `framebuffer_dimensions`: Framebuffer dimensions.
    pub fn record_commands(
        &mut self,
        command_buffer: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        primitives: &Vec<ClippedPrimitive>,
        scale_factor: f32,
        need_srgb_conv: bool,
        framebuffer_dimensions: [u32; 2],
    ) {
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
                    // indicates problem occurred with updating textures...
                    if self.texture_desc_sets.get(&mesh.texture_id).is_none() {
                        error!(
                            "required gui texture no longer exists: {:?}",
                            mesh.texture_id
                        );
                        continue;
                    }

                    // get region of screen to render
                    let scissors = [get_rect_scissor(
                        scale_factor,
                        framebuffer_dimensions,
                        *clip_rect,
                    )];
                    // todo description
                    let (vertices, indices) = self.create_subbuffers(mesh);

                    let desc_set = self
                        .texture_desc_sets
                        .get(&mesh.texture_id)
                        .unwrap()
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
                        .unwrap();
                }
                _ => continue, // don't need to support Primitive::Callback
            }
        }
    }
}
// Private functions
impl GuiRenderer {
    /// Builds the gui rendering graphics pipeline.
    ///
    /// Helper function for [`Self::new`]
    fn create_pipeline(
        device: Arc<Device>,
        swapchain_image_format: Format,
    ) -> Result<Arc<GraphicsPipeline>, RenderManagerError> {
        let vert_shader =
            create_shader_module(device.clone(), "assets/shader_binaries/gui.vert.spv")?;
        let frag_shader =
            create_shader_module(device.clone(), "assets/shader_binaries/gui.frag.spv")?;

        let mut blend = AttachmentBlend::alpha();
        blend.color_source = BlendFactor::One;
        let blend_state = ColorBlendState::new(1).blend(blend);

        Ok(GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<EguiVertex>())
            .vertex_shader(vert_shader.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .fragment_shader(frag_shader.entry_point("main").unwrap(), ())
            .viewport_state(ViewportState::viewport_dynamic_scissor_dynamic(1))
            .color_blend_state(blend_state)
            .rasterization_state(RasterizationState::new().cull_mode(CullMode::None))
            .render_pass(PipelineRenderingCreateInfo {
                color_attachment_formats: vec![Some(swapchain_image_format)],
                ..Default::default()
            })
            .build(device.clone())
            .unwrap())
    }

    /// Creates vertex and index buffer pools.
    ///
    /// Helper function for [`Self::new`]
    fn create_buffer_pools(device: Arc<Device>) -> (CpuBufferPool<EguiVertex>, CpuBufferPool<u32>) {
        // Create vertex and index buffers
        let vertex_buffer_pool = CpuBufferPool::vertex_buffer(device.clone());
        vertex_buffer_pool
            .reserve(VERTEX_BUFFER_SIZE)
            .expect("Failed to reserve vertex buffer memory");
        let index_buffer_pool = CpuBufferPool::new(device, BufferUsage::index_buffer());
        index_buffer_pool
            .reserve(INDEX_BUFFER_SIZE)
            .expect("Failed to reserve index buffer memory");

        (vertex_buffer_pool, index_buffer_pool)
    }

    /// Creates texture sampler.
    ///
    /// Helper function for [`Self::new`]
    fn create_sampler(device: Arc<Device>) -> Arc<Sampler> {
        Sampler::new(
            device.clone(),
            SamplerCreateInfo {
                mag_filter: sampler::Filter::Linear,
                min_filter: sampler::Filter::Linear,
                address_mode: [SamplerAddressMode::ClampToEdge; 3],
                mipmap_mode: SamplerMipmapMode::Linear,
                ..Default::default()
            },
        )
        .unwrap()
    }

    /// Creates a new texture needing to be added for the gui.
    ///
    /// Helper function for [`Self::update_textures`]
    fn create_texture(&mut self, texture_id: egui::TextureId, delta: &egui::epaint::ImageDelta) {
        // Extract pixel data from egui
        let data: Vec<u8> = match &delta.image {
            egui::ImageData::Color(image) => {
                if image.width() * image.height() != image.pixels.len() {
                    warn!("mismatch between gui texture size and texel count"); // todo handle this?
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
        // Create buffer to be copied to the image
        let texture_data_buffer = CpuAccessibleBuffer::from_iter(
            self.device.clone(),
            BufferUsage::transfer_src(),
            false,
            data,
        )
        .unwrap();
        // Create image
        let (img, init) = ImmutableImage::uninitialized(
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
                transfer_src: true,
                sampled: true,
                ..ImageUsage::none()
            },
            Default::default(),
            ImageLayout::ShaderReadOnlyOptimal,
            Some(self.queue.family()),
        )
        .unwrap();
        let font_image = ImageView::new_default(img).unwrap();

        // Create command buffer builder
        let mut cbb = AutoCommandBufferBuilder::primary(
            self.device.clone(),
            self.queue.family(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        // Copy buffer to image
        cbb.copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
            texture_data_buffer,
            init.clone(),
        ))
        .unwrap();

        // Blit texture data to existing image if delta pos exists (e.g. font changed)
        if let Some(pos) = delta.pos {
            if let Some(existing_image) = self.texture_images.get(&texture_id) {
                let src_dims = font_image.image().dimensions();
                let top_left = [pos[0] as u32, pos[1] as u32, 0];
                let bottom_right = [
                    pos[0] as u32 + src_dims.width() as u32,
                    pos[1] as u32 + src_dims.height() as u32,
                    1,
                ];

                cbb.blit_image(BlitImageInfo {
                    src_image_layout: ImageLayout::General,
                    dst_image_layout: ImageLayout::General,
                    regions: [ImageBlit {
                        src_subresource: font_image.image().subresource_layers(),
                        src_offsets: [
                            [0, 0, 0],
                            [src_dims.width() as u32, src_dims.height() as u32, 1],
                        ],
                        dst_subresource: existing_image.image().subresource_layers(),
                        dst_offsets: [top_left, bottom_right],
                        ..Default::default()
                    }]
                    .into(),
                    filter: vulkano::sampler::Filter::Nearest,
                    ..BlitImageInfo::images(
                        font_image.image().clone(),
                        existing_image.image().clone(),
                    )
                })
                .unwrap();
            }
            // Otherwise save the newly created image
        } else {
            let layout = self.pipeline.layout().set_layouts().get(0).unwrap();
            let font_desc_set = self.sampled_image_desc_set(layout, font_image.clone());
            self.texture_desc_sets.insert(texture_id, font_desc_set);
            self.texture_images.insert(texture_id, font_image);
        }
        // Execute command buffer
        let command_buffer = cbb.build().unwrap();
        let finished = command_buffer.execute(self.queue.clone()).unwrap();
        let _fut = finished.then_signal_fence_and_flush().unwrap();
    }

    /// Unregister a texture that is no longer required by the gui.
    ///
    /// Helper function for [`Self::update_textures`]
    fn unregister_image(&mut self, texture_id: egui::TextureId) {
        self.texture_desc_sets.remove(&texture_id);
        self.texture_images.remove(&texture_id);
    }

    fn create_subbuffers(
        &self,
        mesh: &Mesh,
    ) -> (
        Arc<CpuBufferPoolChunk<EguiVertex, Arc<StdMemoryPool>>>,
        Arc<CpuBufferPoolChunk<u32, Arc<StdMemoryPool>>>,
    ) {
        // Copy vertices to buffer
        let v_slice = &mesh.vertices;

        let vertex_chunk = self
            .vertex_buffer_pool
            .chunk(v_slice.into_iter().map(|v| EguiVertex {
                position: [v.pos.x, v.pos.y],
                tex_coords: [v.uv.x, v.uv.y],
                color: [
                    v.color.r() as f32 / 255.0,
                    v.color.g() as f32 / 255.0,
                    v.color.b() as f32 / 255.0,
                    v.color.a() as f32 / 255.0,
                ],
            }))
            .unwrap();

        // Copy indices to buffer
        let i_slice = &mesh.indices;
        let index_chunk = self.index_buffer_pool.chunk(i_slice.clone()).unwrap();

        (vertex_chunk, index_chunk)
    }
    /// Creates a descriptor set for images
    fn sampled_image_desc_set(
        &self,
        layout: &Arc<DescriptorSetLayout>,
        image: Arc<dyn ImageViewAbstract + 'static>,
    ) -> Arc<PersistentDescriptorSet> {
        PersistentDescriptorSet::new(
            layout.clone(),
            [WriteDescriptorSet::image_view_sampler(
                0,
                image.clone(),
                self.sampler.clone(),
            )],
        )
        .unwrap()
    }
}

// ~~~ Helper functions ~~~

/// Caclulates the region of the framebuffer to render a gui element.
fn get_rect_scissor(scale_factor: f32, framebuffer_dimensions: [u32; 2], rect: Rect) -> Scissor {
    let min = rect.min;
    let min = egui::Pos2 {
        x: min.x * scale_factor,
        y: min.y * scale_factor,
    };
    let min = egui::Pos2 {
        x: min.x.clamp(0.0, framebuffer_dimensions[0] as f32),
        y: min.y.clamp(0.0, framebuffer_dimensions[1] as f32),
    };
    let max = rect.max;
    let max = egui::Pos2 {
        x: max.x * scale_factor,
        y: max.y * scale_factor,
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
