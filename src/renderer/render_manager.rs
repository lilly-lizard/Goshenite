use crate::{
    engine::object::objects_delta::ObjectsDelta,
    renderer::{config_renderer::RenderOptions, element_id_reader::ElementAtPoint},
    user_interface::camera::Camera,
};
use egui::{ClippedPrimitive, TexturesDelta};
use std::sync::Arc;
use winit::{event_loop::OwnedDisplayHandle, window::Window};

pub struct RenderManager {
    instance: wgpu::Instance,
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,

    /// Can be set to true with [`Self::set_window_just_resized_flag`] and set to false in [`Self::render_frame`]
    window_just_resized: bool,
}

impl RenderManager {
    pub async fn new(
        display: OwnedDisplayHandle,
        window: Arc<Window>,
        scale_factor: f32,
    ) -> anyhow::Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(
            Box::new(display),
        ));
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();

        let surface_size = window.inner_size();

        let surface = instance.create_surface(window.clone()).unwrap();
        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];

        let mut render_manager = Self {
            instance,
            window,
            device,
            queue,
            surface_size,
            surface,
            surface_format,
            window_just_resized: false,
        };

        render_manager.configure_surface();

        Ok(render_manager)
    }

    pub fn max_2d_image_size(&self) -> u32 {
        self.device.limits().max_texture_dimension_2d
    }

    /// todo
    pub fn render_frame(&mut self, overlay_options: RenderOptions) -> anyhow::Result<()> {
        if self.window_just_resized {
            self.window_just_resized = false;
            self.recreate_surface_and_framebuffers();
        }

        // Create texture view.
        // NOTE: We must handle Timeout because the surface may be unavailable
        // (e.g., when the window is occluded on macOS).
        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => texture,
            wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Timeout => {
                return Ok(())
            }
            wgpu::CurrentSurfaceTexture::Suboptimal(_) | wgpu::CurrentSurfaceTexture::Outdated => {
                self.recreate_surface_and_framebuffers();
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                unreachable!("No error scope registered, so validation errors will panic")
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface = self.instance.create_surface(self.window.clone()).unwrap();
                self.recreate_surface_and_framebuffers();
                return Ok(());
            }
        };
        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                // Without add_srgb_suffix() the image we will be working with
                // might not be "gamma correct".
                format: Some(self.surface_format.add_srgb_suffix()),
                ..Default::default()
            });

        // Renders a GREEN screen
        let mut encoder = self.device.create_command_encoder(&Default::default());
        // Create the renderpass which will clear the screen.
        let renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &texture_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        // If you wanted to call any drawing commands, they would go here.

        // End the renderpass.
        drop(renderpass);

        // Submit the command in the queue to execute
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        surface_texture.present();

        Ok(())
    }

    fn recreate_surface_and_framebuffers(&mut self) {
        self.configure_surface();
    }

    fn configure_surface(&mut self) {
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            // Request compatibility with the sRGB-format texture view we‘re going to create later.
            view_formats: vec![self.surface_format.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.surface_size.width,
            height: self.surface_size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        self.surface.configure(&self.device, &surface_config);
    }

    /// todo
    pub fn update_camera(&mut self, camera: &Camera) -> anyhow::Result<()> {
        Ok(())
    }

    /// todo
    pub fn update_objects(&mut self, objects_delta: ObjectsDelta) -> anyhow::Result<()> {
        Ok(())
    }

    /// todo
    pub fn update_gui_textures(
        &mut self,
        textures_delta: Vec<TexturesDelta>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    /// todo
    pub fn set_gui_primitives(&mut self, gui_primitives: Vec<ClippedPrimitive>) {}

    pub fn resize_surface(&mut self, new_inner_window_size: winit::dpi::PhysicalSize<u32>) {
        self.window_just_resized = true;
        self.surface_size = new_inner_window_size;
    }

    /// todo
    pub fn set_scale_factor(&mut self, scale_factor: f32) {}

    /// todo
    pub fn get_element_at_screen_coordinate(
        &mut self,
        screen_coordinate: [f32; 2],
    ) -> anyhow::Result<Option<ElementAtPoint>> {
        Ok(None)
    }
}
