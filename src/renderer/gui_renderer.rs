// shout out to https://github.com/hakolao/egui_winit_vulkano

use crate::renderer::render_manager::{create_shader_module, RenderManager, RenderManagerError};
use crate::shaders::shader_interfaces;
use egui::epaint::ClippedShape;
use egui::{epaint::Primitive, ClippedPrimitive};
use egui::{Mesh, Rect};
use std::{collections::HashMap, sync::Arc};
use vulkano::buffer::cpu_pool::CpuBufferPoolChunk;
use vulkano::buffer::{CpuBufferPool, TypedBufferAccess};
use vulkano::pipeline::graphics::viewport::{Scissor, Viewport};
use vulkano::pipeline::PipelineBindPoint;
use vulkano::DeviceSize;
use vulkano::{
    buffer::cpu_access::CpuAccessibleBuffer,
    buffer::BufferUsage,
    command_buffer::{
        self, AutoCommandBufferBuilder, PrimaryCommandBuffer, SecondaryAutoCommandBuffer,
    },
    descriptor_set::{layout::DescriptorSetLayout, PersistentDescriptorSet, WriteDescriptorSet},
    device::{Device, Queue},
    format::Format,
    image::{self, ImageAccess, ImageViewAbstract},
    memory::pool::StdMemoryPool,
    pipeline::{
        graphics::{
            color_blend::{AttachmentBlend, BlendFactor, ColorBlendState},
            input_assembly::InputAssemblyState,
            rasterization::{CullMode, RasterizationState},
            render_pass::PipelineRenderingCreateInfo,
            viewport::ViewportState,
        },
        graphics::{vertex_input::BuffersDefinition, GraphicsPipeline},
        Pipeline,
    },
    render_pass::Subpass,
    sampler::{self, Sampler, SamplerAddressMode, SamplerCreateInfo, SamplerMipmapMode},
    sync::GpuFuture,
};
use winit::window::Window;

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
    context: egui::Context,
    window_state: egui_winit::State,
    window: Arc<Window>,

    pipeline: Arc<GraphicsPipeline>,
    sampler: Arc<Sampler>,
    vertex_buffer_pool: CpuBufferPool<EguiVertex>,
    index_buffer_pool: CpuBufferPool<u32>,

    shapes: Vec<ClippedShape>,
    textures_delta: egui::TexturesDelta,
    texture_images: HashMap<egui::TextureId, Arc<dyn ImageViewAbstract + Send + Sync + 'static>>,
    texture_desc_sets: HashMap<egui::TextureId, Arc<PersistentDescriptorSet>>,
}

impl GuiRenderer {
    pub fn new(
        window: Arc<winit::window::Window>,
        physical_device: &vulkano::device::physical::PhysicalDevice,
        device: Arc<Device>,
        swapchain_image_format: Format,
    ) -> Result<Self, RenderManagerError> {
        let window_state = egui_winit::State::new(
            physical_device.properties().max_image_array_layers as usize,
            window.as_ref(),
        );
        let pipeline = Self::create_pipeline(device.clone(), swapchain_image_format)?;
        let sampler = Self::create_sampler(device.clone());
        let (vertex_buffer_pool, index_buffer_pool) = Self::create_buffers(device.clone());

        Ok(Self {
            context: Default::default(),
            window_state,
            window,
            pipeline,
            sampler,
            vertex_buffer_pool,
            index_buffer_pool,
            shapes: vec![],
            textures_delta: Default::default(),
            texture_images: HashMap::default(),
            texture_desc_sets: HashMap::default(),
        })
    }
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
    fn create_buffers(device: Arc<Device>) -> (CpuBufferPool<EguiVertex>, CpuBufferPool<u32>) {
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

    /// Updates context state by winit window event.
    /// Returns `true` if egui wants exclusive use of this event
    /// (e.g. a mouse click on an egui window, or entering text into a text field).
    /// For instance, if you use egui for a game, you want to first call this
    /// and only when this returns `false` pass on the events to your game.
    ///
    /// Note that egui uses `tab` to move focus between elements, so this will always return `true` for tabs.
    pub fn update(&mut self, event: &winit::event::WindowEvent<'_>) -> bool {
        self.window_state.on_event(&self.context, event)
    }

    /// Begins Egui frame & determines what will be drawn later. This must be called before draw, and after `update` (winit event).
    pub fn immediate_ui(
        &mut self,
        device: Arc<Device>,
        queue: Arc<Queue>,
        command_buffer_builder: AutoCommandBufferBuilder<SecondaryAutoCommandBuffer>,
        framebuffer_dimensions: [u32; 2],
        need_srgb_conv: bool,
    ) {
        let raw_input = self.window_state.take_egui_input(self.window.as_ref());
        self.context.begin_frame(raw_input);

        // set new layout
        self.layout();

        // update resources and command buffer
        self.update_renderer(
            device,
            queue,
            command_buffer_builder,
            framebuffer_dimensions,
            need_srgb_conv,
        );
    }

    fn layout(&mut self) {
        egui::Window::new("Mah Tree")
            .resizable(true)
            .vscroll(true)
            .hscroll(true)
            .show(&self.context, |ui| {
                ui.heading("hello egui!");
            });
    }

    fn update_renderer(
        &mut self,
        device: Arc<Device>,
        queue: Arc<Queue>,
        mut command_buffer_builder: AutoCommandBufferBuilder<SecondaryAutoCommandBuffer>,
        framebuffer_dimensions: [u32; 2],
        need_srgb_conv: bool,
    ) -> SecondaryAutoCommandBuffer {
        self.end_frame();

        let shapes = std::mem::take(&mut self.shapes);
        let textures_delta = std::mem::take(&mut self.textures_delta);
        let clipped_meshes = self.context.tessellate(shapes);

        for (id, image_delta) in &textures_delta.set {
            self.update_texture(device.clone(), queue.clone(), *id, image_delta);
        }

        self.record_commands(
            self.window_state.pixels_per_point(),
            need_srgb_conv,
            &clipped_meshes,
            framebuffer_dimensions,
            &mut command_buffer_builder,
        );
        let command_buffer = command_buffer_builder.build().unwrap();

        for &id in &textures_delta.free {
            self.unregister_image(id);
        }

        command_buffer
    }

    fn record_commands(
        &mut self,
        scale_factor: f32,
        need_srgb_conv: bool,
        clipped_meshes: &[ClippedPrimitive],
        framebuffer_dimensions: [u32; 2],
        builder: &mut AutoCommandBufferBuilder<SecondaryAutoCommandBuffer>,
    ) {
        let push_constants = shader_interfaces::GuiPc::new(
            [
                framebuffer_dimensions[0] as f32 / scale_factor,
                framebuffer_dimensions[1] as f32 / scale_factor,
            ],
            need_srgb_conv,
        );

        for ClippedPrimitive {
            clip_rect,
            primitive,
        } in clipped_meshes
        {
            match primitive {
                Primitive::Mesh(mesh) => {
                    // Nothing to draw if we don't have vertices & indices
                    if mesh.vertices.is_empty() || mesh.indices.is_empty() {
                        continue;
                    }
                    if self.texture_desc_sets.get(&mesh.texture_id).is_none() {
                        eprintln!("This texture no longer exists {:?}", mesh.texture_id);
                        continue;
                    }

                    let scissors = vec![self.get_rect_scissor(
                        scale_factor,
                        framebuffer_dimensions,
                        *clip_rect,
                    )];

                    let (vertices, indices) = self.create_subbuffers(mesh);

                    let desc_set = self
                        .texture_desc_sets
                        .get(&mesh.texture_id)
                        .unwrap()
                        .clone();
                    builder
                        .bind_pipeline_graphics(self.pipeline.clone())
                        .set_viewport(
                            0,
                            vec![Viewport {
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
                _ => continue,
            }
        }
    }

    fn get_rect_scissor(
        &self,
        scale_factor: f32,
        framebuffer_dimensions: [u32; 2],
        rect: Rect,
    ) -> Scissor {
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

    fn end_frame(&mut self) {
        let egui::FullOutput {
            platform_output,
            needs_repaint: _r,
            textures_delta,
            shapes,
        } = self.context.end_frame();

        self.window_state.handle_platform_output(
            self.window.as_ref(),
            &self.context,
            platform_output,
        );
        self.shapes = shapes;
        self.textures_delta = textures_delta;
    }

    fn update_texture(
        &mut self,
        device: Arc<Device>,
        queue: Arc<Queue>,
        texture_id: egui::TextureId,
        delta: &egui::epaint::ImageDelta,
    ) {
        // Extract pixel data from egui
        let data: Vec<u8> = match &delta.image {
            egui::ImageData::Color(image) => {
                assert_eq!(
                    image.width() * image.height(),
                    image.pixels.len(),
                    "Mismatch between texture size and texel count"
                );
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
            device.clone(),
            BufferUsage::transfer_src(),
            false,
            data,
        )
        .unwrap();
        // Create image
        let (img, init) = image::ImmutableImage::uninitialized(
            device.clone(),
            vulkano::image::ImageDimensions::Dim2d {
                width: delta.image.width() as u32,
                height: delta.image.height() as u32,
                array_layers: 1,
            },
            Format::R8G8B8A8_SRGB,
            vulkano::image::MipmapsCount::One,
            image::ImageUsage {
                transfer_dst: true,
                transfer_src: true,
                sampled: true,
                ..image::ImageUsage::none()
            },
            Default::default(),
            image::ImageLayout::ShaderReadOnlyOptimal,
            Some(queue.family()),
        )
        .unwrap();
        let font_image = image::view::ImageView::new_default(img).unwrap();

        // Create command buffer builder
        let mut cbb = command_buffer::AutoCommandBufferBuilder::primary(
            device.clone(),
            queue.family(),
            command_buffer::CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        // Copy buffer to image
        cbb.copy_buffer_to_image(command_buffer::CopyBufferToImageInfo::buffer_image(
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

                cbb.blit_image(command_buffer::BlitImageInfo {
                    src_image_layout: image::ImageLayout::General,
                    dst_image_layout: image::ImageLayout::General,
                    regions: [command_buffer::ImageBlit {
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
                    ..command_buffer::BlitImageInfo::images(
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
        let finished = command_buffer.execute(queue.clone()).unwrap();
        let _fut = finished.then_signal_fence_and_flush().unwrap();
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

    /// Unregister user texture.
    fn unregister_image(&mut self, texture_id: egui::TextureId) {
        self.texture_desc_sets.remove(&texture_id);
        self.texture_images.remove(&texture_id);
    }
}
