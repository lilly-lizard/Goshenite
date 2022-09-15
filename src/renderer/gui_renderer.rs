// shout out to https://github.com/hakolao/egui_winit_vulkano

use crate::renderer::render_manager::{create_shader_module, RenderManager, RenderManagerError};
use egui::epaint;
use std::{collections::HashMap, hash::Hash, sync::Arc};
use vulkano::{
    buffer::cpu_access::CpuAccessibleBuffer,
    buffer::BufferUsage,
    command_buffer::{self, PrimaryCommandBuffer},
    descriptor_set::{layout::DescriptorSetLayout, PersistentDescriptorSet, WriteDescriptorSet},
    format::Format,
    image::{self, ImageAccess, ImageViewAbstract},
    pipeline::{
        graphics::{
            color_blend::{AttachmentBlend, BlendFactor, ColorBlendState},
            input_assembly::InputAssemblyState,
            rasterization::{CullMode, RasterizationState},
            viewport::ViewportState,
        },
        graphics::{vertex_input::BuffersDefinition, GraphicsPipeline},
        Pipeline,
    },
    render_pass::Subpass,
    sampler::{self, Sampler, SamplerAddressMode, SamplerCreateInfo, SamplerMipmapMode},
    swapchain,
    sync::GpuFuture,
};
use winit::window::Window;

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

    shapes: Vec<epaint::ClippedShape>,
    textures_delta: egui::TexturesDelta,
    texture_images: HashMap<egui::TextureId, Arc<dyn ImageViewAbstract + Send + Sync + 'static>>,
    texture_desc_sets: HashMap<egui::TextureId, Arc<PersistentDescriptorSet>>,
}

impl GuiRenderer {
    pub fn new(
        window: Arc<winit::window::Window>,
        physical_device: &vulkano::device::physical::PhysicalDevice,
        render_manager: &RenderManager,
        subpass: Subpass,
    ) -> Result<Self, RenderManagerError> {
        let window_state = egui_winit::State::new(
            physical_device.properties().max_image_array_layers as usize,
            window.as_ref(),
        );

        let vert_shader = create_shader_module(
            render_manager.device.clone(),
            "assets/shader_binaries/gui.vert.spv",
        )?;
        let frag_shader = create_shader_module(
            render_manager.device.clone(),
            "assets/shader_binaries/gui.frag.spv",
        )?;

        let mut blend = AttachmentBlend::alpha();
        blend.color_source = BlendFactor::One;
        let blend_state = ColorBlendState::new(1).blend(blend);

        let pipeline = GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<EguiVertex>())
            .vertex_shader(vert_shader.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .fragment_shader(frag_shader.entry_point("main").unwrap(), ())
            .viewport_state(ViewportState::viewport_dynamic_scissor_dynamic(1))
            .color_blend_state(blend_state)
            .rasterization_state(RasterizationState::new().cull_mode(CullMode::None))
            .render_pass(subpass)
            .build(render_manager.queue.device().clone())
            .unwrap();

        let sampler = Sampler::new(
            render_manager.queue.device().clone(),
            SamplerCreateInfo {
                mag_filter: sampler::Filter::Linear,
                min_filter: sampler::Filter::Linear,
                address_mode: [SamplerAddressMode::ClampToEdge; 3],
                mipmap_mode: SamplerMipmapMode::Linear,
                ..Default::default()
            },
        )
        .unwrap();

        Ok(Self {
            context: Default::default(),
            window_state,
            window,
            pipeline,
            sampler,
            shapes: vec![],
            textures_delta: Default::default(),
            texture_images: HashMap::default(),
            texture_desc_sets: HashMap::default(),
        })
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
    pub fn immediate_ui(&mut self) {
        let raw_input = self.window_state.take_egui_input(self.window.as_ref());
        self.context.begin_frame(raw_input);
        // Render Egui
        self.layout();
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

    pub fn draw_cmds(&mut self, render_manager: &RenderManager) {
        self.end_frame();

        let shapes = std::mem::take(&mut self.shapes);
        let textures_delta = std::mem::take(&mut self.textures_delta);
        let clipped_meshes = self.context.tessellate(shapes);

        for (id, image_delta) in &textures_delta.set {
            self.update_texture(render_manager, *id, image_delta);
        }

        todo!();
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
        render_manager: &RenderManager,
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
            render_manager.device.clone(),
            BufferUsage::transfer_src(),
            false,
            data,
        )
        .unwrap();
        // Create image
        let (img, init) = image::ImmutableImage::uninitialized(
            render_manager.device.clone(),
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
            Some(render_manager.queue.family()),
        )
        .unwrap();
        let font_image = image::view::ImageView::new_default(img).unwrap();

        // Create command buffer builder
        let mut cbb = command_buffer::AutoCommandBufferBuilder::primary(
            render_manager.device.clone(),
            render_manager.queue.family(),
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
        let finished = command_buffer
            .execute(render_manager.queue.clone())
            .unwrap();
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
}
