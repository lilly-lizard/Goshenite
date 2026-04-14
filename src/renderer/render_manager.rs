use crate::{
    engine::object::objects_delta::ObjectsDelta,
    renderer::{config_renderer::RenderOptions, element_id_reader::ElementAtPoint},
    user_interface::camera::Camera,
};
use anyhow::Context;
use egui::{ClippedPrimitive, TexturesDelta};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::{borrow::Cow, sync::Arc};
use winit::{event_loop::OwnedDisplayHandle, window::Window};

pub struct RenderManager {
    instance: wgpu::Instance,
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    render_pipeline: wgpu::RenderPipeline,

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

        let surface_size = window.inner_size();
        let surface = instance
            .create_surface(window.clone())
            .context("creating render surface")?;

        // physical device
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                // Request an adapter which can render to our surface
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .context("finding graphics device/driver")?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                // make sure we use the texture resolution limits from the adapter,
                // so we can support images the size of the swapchain.
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
            })
            .await
            .context("creating virtual device and render queue")?;

        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];

        let triangle_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("triangle"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("triangle"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });

        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let swapchain_format = swapchain_capabilities.formats[0];

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("triangle"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &triangle_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &triangle_shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(swapchain_format.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let mut render_manager = Self {
            instance,
            window,
            device,
            queue,
            surface_size,
            surface,
            surface_format,
            window_just_resized: false,
            render_pipeline,
        };

        render_manager.configure_surface();

        Ok(render_manager)
    }

    pub fn max_2d_image_size(&self) -> u32 {
        self.device.limits().max_texture_dimension_2d
    }

    pub fn render_frame(&mut self, overlay_options: RenderOptions) -> anyhow::Result<()> {
        if self.window_just_resized {
            self.window_just_resized = false;
            self.reconfigure_surface_and_framebuffers();
        }

        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => texture,
            wgpu::CurrentSurfaceTexture::Occluded => {
                debug!("window occluded, skipping frame render");
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Timeout => {
                debug!("get render surface framebuffer timeout");
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Suboptimal(_) | wgpu::CurrentSurfaceTexture::Outdated => {
                info!("suboptimal or outdated render surface, recreating framebuffers");
                self.reconfigure_surface_and_framebuffers();
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                todo!("figure out how validation errors work");
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                warn!("render surface lost, attempting to recreate");
                self.surface = self
                    .instance
                    .create_surface(self.window.clone())
                    .context("recreating render surface after surface/device lost")?;
                self.reconfigure_surface_and_framebuffers();
                return Ok(());
            }
        };

        let surface_texture_view =
            surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor {
                    format: Some(self.surface_format.add_srgb_suffix()),
                    ..Default::default()
                });

        // begin the renderpass
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let mut renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("per frame render"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &surface_texture_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    // Renders a GREEN screen
                    load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        renderpass.set_pipeline(&self.render_pipeline);
        renderpass.draw(0..3, 0..1);

        // end the renderpass
        drop(renderpass);

        // submit the render commands
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        surface_texture.present();

        Ok(())
    }

    fn reconfigure_surface_and_framebuffers(&mut self) {
        self.configure_surface();
    }

    fn configure_surface(&mut self) {
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            // request compatibility with the sRGB-format texture view we‘re going to create later.
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
