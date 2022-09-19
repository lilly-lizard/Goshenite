use super::gui_renderer::GuiRenderer;
use crate::camera::Camera;
use crate::config;
use crate::shaders::shader_interfaces;
use egui::ClippedPrimitive;
use log::{debug, error, info, warn};
use std::{error, fmt, sync::Arc};
use vulkano::device::Queue;
use vulkano::instance::Instance;
use vulkano::pipeline::graphics::render_pass::PipelineRenderingCreateInfo;
use vulkano::pipeline::{ComputePipeline, GraphicsPipeline};
use vulkano::sampler::Sampler;
use vulkano::swapchain::{Surface, Swapchain, SwapchainCreationError};
use vulkano::{
    command_buffer,
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::physical::{PhysicalDevice, PhysicalDeviceType, SurfacePropertiesError},
    device::{self, Device},
    format::Format,
    image::{view::ImageView, ImageAccess, ImageUsage, StorageImage, SwapchainImage},
    instance,
    instance::debug::{
        DebugUtilsMessageSeverity, DebugUtilsMessageType, DebugUtilsMessenger,
        DebugUtilsMessengerCreateInfo,
    },
    instance::LayersListError,
    pipeline,
    pipeline::graphics::viewport::{Viewport, ViewportState},
    pipeline::Pipeline,
    render_pass::{LoadOp, StoreOp},
    sampler,
    shader::ShaderModule,
    swapchain,
    sync::{self, FlushError, GpuFuture},
};
use winit::window::Window;

/// Contains Vulkan resources and methods to manage rendering
pub struct RenderManager {
    device: Arc<Device>,
    queue: Arc<Queue>,
    _debug_callback: Option<DebugUtilsMessenger>,

    surface: Arc<Surface<Arc<Window>>>,
    swapchain: Arc<Swapchain<Arc<Window>>>,
    swapchain_image_views: Vec<Arc<ImageView<SwapchainImage<Arc<Window>>>>>,

    render_image: Arc<ImageView<StorageImage>>,
    render_image_sampler: Arc<Sampler>,

    render_pipeline: Arc<ComputePipeline>,
    render_desc_set: Arc<PersistentDescriptorSet>,
    work_group_size: [u32; 2],
    work_group_count: [u32; 3],
    blit_pipeline: Arc<GraphicsPipeline>,
    blit_desc_set: Arc<PersistentDescriptorSet>,
    viewport: Viewport,

    gui_renderer: GuiRenderer,

    future_previous_frame: Option<Box<dyn GpuFuture>>, // todo description
    /// indicates that the swapchain needs to be recreated next frame
    recreate_swapchain: bool,
}
// Public functions
impl RenderManager {
    /// Initializes Vulkan resources. If renderer fails to initialize, returns a string explanation.
    pub fn new(window: Arc<Window>) -> Result<Self, RenderManagerError> {
        let mut instance_extensions = vulkano_win::required_extensions();
        let mut instance_layers: Vec<String> = Vec::new();

        // check for validation layer/debug callback support
        let enable_debug_callback =
            if add_debug_validation(&mut instance_extensions, &mut instance_layers).is_ok() {
                info!("enabling Vulkan validation layers and debug callback");
                true
            } else {
                warn!("validation layer debug callback requested but cannot be enabled");
                false
            };

        // create instance
        let instance = Instance::new(instance::InstanceCreateInfo {
            enabled_extensions: instance_extensions,
            enumerate_portability: true, // enable enumerating devices that use non-conformant vulkan implementations. (ex. MoltenVK)
            enabled_layers: instance_layers,
            ..Default::default()
        })
        .unrec_err("Failed to create vulkan instance")?;

        // setup debug callbacks
        let debug_callback = if enable_debug_callback {
            unsafe {
                DebugUtilsMessenger::new(
                    instance.clone(),
                    DebugUtilsMessengerCreateInfo {
                        message_severity: DebugUtilsMessageSeverity {
                            error: true,
                            warning: true,
                            information: true,
                            verbose: false,
                        },
                        message_type: DebugUtilsMessageType::all(),
                        ..DebugUtilsMessengerCreateInfo::user_callback(Arc::new(|msg| {
                            vulkan_callback::process_debug_callback(msg)
                        }))
                    },
                )
                .ok()
            }
        } else {
            None
        };

        let surface = vulkano_win::create_surface_from_winit(window.clone(), instance.clone())
            .unrec_err("failed to create vulkan surface")?;

        let device_extensions = device::DeviceExtensions {
            khr_swapchain: true,
            ..device::DeviceExtensions::none()
        };

        // print available devices
        debug!("Available Vulkan physical devices:");
        for pd in PhysicalDevice::enumerate(&instance) {
            debug!("\t{}", pd.properties().device_name);
        }
        // choose physical device and queue family
        let (physical_device, queue_family) = PhysicalDevice::enumerate(&instance)
            // filter for vulkan version support
            .filter(|&p| {
                p.api_version()
                    >= vulkano::Version::major_minor(config::VULKAN_VER_MAJ, config::VULKAN_VER_MIN)
            })
            // filter for required device extensions
            .filter(|&p| p.supported_extensions().is_superset_of(&device_extensions))
            // filter for queue support
            .filter_map(|p| {
                p.queue_families()
                    .find(|&q| {
                        q.supports_compute()
                            && q.supports_graphics()
                            && q.supports_surface(&surface).unwrap_or(false)
                    })
                    .map(|q| (p, q))
            })
            // preference of device type
            .max_by_key(|(p, _)| match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 4,
                PhysicalDeviceType::IntegratedGpu => 3,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 1,
                PhysicalDeviceType::Other => 0,
            })
            .unrec_err("no suitable physical device available")?;
        info!(
            "Using Vulkan device: {} (type: {:?})",
            physical_device.properties().device_name,
            physical_device.properties().device_type,
        );

        let (device, mut queues) = device::Device::new(
            physical_device,
            device::DeviceCreateInfo {
                enabled_extensions: device_extensions,
                enabled_features: device::Features {
                    dynamic_rendering: true,
                    ..device::Features::none()
                },
                queue_create_infos: vec![device::QueueCreateInfo::family(queue_family)],
                ..Default::default()
            },
        )
        .unrec_err("failed to create vulkan device")?;

        let queue = queues.next().expect(
            "vulkano::device::Device::new has an assert to ensure at least 1 queue gets created",
        );

        // create swapchain and images
        let (swapchain, swapchain_images) =
            Self::create_swapchain(device.clone(), physical_device, surface.clone())?;
        debug!(
            "initial swapchain image size = {:?}",
            swapchain_images[0].dimensions()
        );

        // dynamic viewport
        let viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [
                swapchain_images[0].dimensions().width() as f32,
                swapchain_images[0].dimensions().height() as f32,
            ],
            depth_range: 0.0..1.0,
        };

        // calculate work group size and count for render compute shader
        let work_group_size = [
            std::cmp::min(
                config::DEFAULT_WORK_GROUP_SIZE[0],
                physical_device.properties().max_compute_work_group_size[0],
            ),
            std::cmp::min(
                config::DEFAULT_WORK_GROUP_SIZE[1],
                physical_device.properties().max_compute_work_group_size[1],
            ),
        ];
        let work_group_count = calc_work_group_count(
            physical_device,
            swapchain_images[0].dimensions().width_height(),
            work_group_size,
        )?;

        // create swapchain image views
        let swapchain_image_views: Result<Vec<_>, _> = swapchain_images
            .iter()
            .map(|image| ImageView::new_default(image.clone()))
            .collect();
        let swapchain_image_views =
            swapchain_image_views.unrec_err("failed to create swapchain image view(s)")?;

        // compute shader render target
        let render_image = Self::create_render_image(
            queue.clone(),
            swapchain_images[0].dimensions().width_height(),
        )?;
        let render_image_sampler = Self::create_render_image_sampler(device.clone())?;

        let render_pipeline = Self::create_compute_pipeline(device.clone(), work_group_size)?;
        let render_desc_set =
            Self::create_compute_desc_set(render_pipeline.clone(), render_image.clone())?;

        let blit_pipeline = Self::create_blit_pipeline(device.clone(), swapchain.image_format())?;
        let blit_desc_set = Self::create_blit_desc_set(
            blit_pipeline.clone(),
            render_image.clone(),
            render_image_sampler.clone(),
        )?;

        let gui_renderer =
            GuiRenderer::new(device.clone(), queue.clone(), swapchain.image_format())?;

        let future_previous_frame = Some(sync::now(device.clone()).boxed());
        let recreate_swapchain = false;

        Ok(RenderManager {
            _debug_callback: debug_callback,
            device,
            queue,
            surface,
            swapchain,
            swapchain_image_views,
            viewport,
            render_image,
            render_image_sampler,
            render_pipeline,
            blit_pipeline,
            render_desc_set,
            blit_desc_set,
            work_group_size,
            work_group_count,
            gui_renderer,
            future_previous_frame,
            recreate_swapchain,
        })
    }

    /// bruh
    pub fn max_image_array_layers(&self) -> u32 {
        self.device
            .physical_device()
            .properties()
            .max_image_array_layers
    }

    /// Returns a mutable reference to the gui renderer so its resources can be updated by the gui
    pub fn gui_renderer(&mut self) -> &mut GuiRenderer {
        &mut self.gui_renderer
    }

    /// Submits Vulkan commands for rendering a frame.
    pub fn render_frame(
        &mut self,
        window_resize: bool,
        gui_primitives: &Vec<ClippedPrimitive>,
        gui_scale_factor: f32,
        camera: Camera,
    ) -> Result<(), RenderManagerError> {
        use RenderManagerError::Unrecoverable;

        // checks for submission finish and free locks on gpu resources
        self.future_previous_frame
            .as_mut()
            .unwrap()
            .cleanup_finished();

        self.recreate_swapchain = self.recreate_swapchain || window_resize;
        if self.recreate_swapchain {
            // recreate swapchain and skip frame render
            return self.recreate_swapchain();
        }

        // blocks when no images currently available (all have been submitted already)
        let (image_num, suboptimal, acquire_future) =
            match swapchain::acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => r,
                Err(swapchain::AcquireError::OutOfDate) => {
                    self.recreate_swapchain = true;
                    // recreate swapchain and skip frame render
                    return self.recreate_swapchain();
                }
                Err(e) => {
                    // todo other error handling
                    return Err(Unrecoverable(format!(
                        "Failed to acquire next image: {:?}",
                        e
                    )));
                }
            };
        if suboptimal {
            self.recreate_swapchain = true;
        }

        // update gui
        let need_srgb_conv = false; // todo

        let render_push_constants = shader_interfaces::CameraPc::new(
            glam::Mat4::inverse(&(camera.proj_matrix() * camera.view_matrix())),
            camera.position(),
        );

        // record command buffer
        let mut builder = command_buffer::AutoCommandBufferBuilder::primary(
            self.device.clone(),
            self.queue.family(),
            command_buffer::CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();
        builder
            // compute shader scene render
            .bind_pipeline_compute(self.render_pipeline.clone())
            .bind_descriptor_sets(
                pipeline::PipelineBindPoint::Compute,
                self.render_pipeline.layout().clone(),
                0,
                self.render_desc_set.clone(),
            )
            .push_constants(
                self.render_pipeline.layout().clone(),
                0,
                render_push_constants,
            )
            .dispatch(self.work_group_count)
            .unwrap()
            // begin render pass
            .begin_rendering(command_buffer::RenderingInfo {
                color_attachments: vec![Some(command_buffer::RenderingAttachmentInfo {
                    load_op: LoadOp::Clear,
                    store_op: StoreOp::Store,
                    clear_value: Some([0.0, 1.0, 0.0, 1.0].into()),
                    ..command_buffer::RenderingAttachmentInfo::image_view(
                        self.swapchain_image_views[image_num].clone(),
                    )
                })],
                ..Default::default()
            })
            .unwrap()
            // write the render to the swapchain image
            .set_viewport(0, [self.viewport.clone()])
            .bind_pipeline_graphics(self.blit_pipeline.clone())
            .bind_descriptor_sets(
                pipeline::PipelineBindPoint::Graphics,
                self.blit_pipeline.layout().clone(),
                0,
                self.blit_desc_set.clone(),
            )
            .draw(3, 1, 0, 0)
            .unwrap();
        // render gui
        self.gui_renderer.record_commands(
            &mut builder,
            gui_primitives,
            gui_scale_factor,
            need_srgb_conv,
            [
                self.viewport.dimensions[0] as u32,
                self.viewport.dimensions[1] as u32,
            ],
        );
        // end render pass
        builder.end_rendering().unwrap();
        let command_buffer = builder.build().unwrap();

        // submit
        let future = self
            .future_previous_frame
            .take()
            .unwrap()
            .join(acquire_future)
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(self.queue.clone(), self.swapchain.clone(), image_num)
            .then_signal_fence_and_flush();

        match future {
            Ok(future) => {
                self.future_previous_frame = Some(future.boxed());
            }
            Err(FlushError::OutOfDate) => {
                self.recreate_swapchain = true;
                self.future_previous_frame = Some(sync::now(self.device.clone()).boxed());
            }
            Err(e) => {
                error!("Failed to flush future: {:?}", e);
                self.future_previous_frame = Some(sync::now(self.device.clone()).boxed());
            }
        }
        Ok(())
    }
}
// Private functions
impl RenderManager {
    fn create_swapchain(
        device: Arc<Device>,
        physical_device: PhysicalDevice,
        surface: Arc<Surface<Arc<Window>>>,
    ) -> Result<
        (
            Arc<Swapchain<Arc<Window>>>,
            Vec<Arc<SwapchainImage<Arc<Window>>>>,
        ),
        RenderManagerError,
    > {
        use RenderManagerError::Unrecoverable;

        // todo prefer sRGB (linux sRGB)
        let image_format = match physical_device.surface_formats(&surface, Default::default()) {
            Ok(x) => x,
            Err(SurfacePropertiesError::SurfaceLost) => {
                return Err(RenderManagerError::SurfaceLost.log())
            }
            Err(e) => {
                return Err(Unrecoverable(format!("failed to get surface format: {:?}", e)).log())
            }
        }
        .get(0)
        .expect("vulkan driver should support at least 1 surface format... right?")
        .0;
        debug!("swapchain image format = {:?}", image_format);

        let surface_capabilities = match physical_device
            .surface_capabilities(&surface, Default::default())
        {
            Ok(x) => x,
            Err(SurfacePropertiesError::SurfaceLost) => {
                return Err(RenderManagerError::SurfaceLost.log())
            }
            Err(e) => {
                return Err(
                    Unrecoverable(format!("failed to get surface capabilities: {:?}", e,)).log(),
                )
            }
        };
        let composite_alpha = surface_capabilities
            .supported_composite_alpha
            .iter()
            .max_by_key(|c| match c {
                swapchain::CompositeAlpha::PostMultiplied => 4,
                swapchain::CompositeAlpha::Inherit => 3,
                swapchain::CompositeAlpha::Opaque => 2,
                swapchain::CompositeAlpha::PreMultiplied => 1, // because cbf implimenting this logic
                _ => 0,
            })
            .expect("surface should support at least 1 composite mode... right?");
        debug!("swapchain composite alpha = {:?}", composite_alpha);

        let mut present_modes = match physical_device.surface_present_modes(&surface) {
            Ok(x) => x,
            Err(SurfacePropertiesError::SurfaceLost) => {
                return Err(RenderManagerError::SurfaceLost.log())
            }
            Err(e) => {
                return Err(
                    Unrecoverable(format!("failed to get surface capabilities: {:?}", e,)).log(),
                )
            }
        };
        let present_mode = present_modes
            .find(|&pm| pm == swapchain::PresentMode::Mailbox)
            .unwrap_or(swapchain::PresentMode::Fifo);
        debug!("swapchain present mode = {:?}", present_mode);

        match swapchain::Swapchain::new(
            device.clone(),
            surface.clone(),
            swapchain::SwapchainCreateInfo {
                min_image_count: surface_capabilities.min_image_count,
                image_extent: surface.window().inner_size().into(),
                image_usage: ImageUsage::color_attachment(),
                image_format: Some(image_format),
                composite_alpha,
                present_mode,
                ..Default::default()
            },
        ) {
            Ok(x) => Ok(x),
            Err(e) => return Err(e.into()),
        }
    }

    fn create_render_image(
        queue: Arc<Queue>,
        size: [u32; 2],
    ) -> Result<Arc<ImageView<StorageImage>>, RenderManagerError> {
        // format must match what's specified in the compute shader layout
        let render_image_format = Format::R8G8B8A8_UNORM;
        StorageImage::general_purpose_image_view(
            queue,
            size,
            render_image_format,
            ImageUsage {
                storage: true,
                sampled: true,
                ..ImageUsage::none()
            },
        )
        .unrec_err("failed to create render image")
    }

    fn create_render_image_sampler(
        device: Arc<Device>,
    ) -> Result<Arc<Sampler>, RenderManagerError> {
        sampler::Sampler::new(
            device,
            sampler::SamplerCreateInfo {
                mag_filter: sampler::Filter::Linear,
                min_filter: sampler::Filter::Linear,
                address_mode: [sampler::SamplerAddressMode::Repeat; 3],
                ..Default::default()
            },
        )
        .unrec_err("failed to create sampler")
    }

    fn create_compute_pipeline(
        device: Arc<Device>,
        work_group_size: [u32; 2],
    ) -> Result<Arc<ComputePipeline>, RenderManagerError> {
        let render_shader =
            create_shader_module(device.clone(), "assets/shader_binaries/render.comp.spv")?;

        let compute_spec_constant = shader_interfaces::ComputeSpecConstant {
            local_size_x: work_group_size[0],
            local_size_y: work_group_size[1],
        };
        pipeline::ComputePipeline::new(
            device.clone(),
            render_shader
                .entry_point("main")
                .unrec_err("no main in render.comp")?,
            &compute_spec_constant,
            None,
            |_| {},
        )
        .unrec_err("failed to create render compute pipeline")
    }

    fn create_compute_desc_set(
        render_pipeline: Arc<ComputePipeline>,
        render_image: Arc<ImageView<StorageImage>>,
    ) -> Result<Arc<PersistentDescriptorSet>, RenderManagerError> {
        PersistentDescriptorSet::new(
            render_pipeline
                .layout()
                .set_layouts()
                .get(shader_interfaces::descriptor::SET_RENDER_COMP)
                .unwrap()
                .to_owned(),
            [WriteDescriptorSet::image_view(
                shader_interfaces::descriptor::BINDING_IMAGE,
                render_image,
            )],
        )
        .unrec_err("unable to create render compute shader descriptor set")
    }

    fn create_blit_pipeline(
        device: Arc<Device>,
        swapchain_image_format: Format,
    ) -> Result<Arc<GraphicsPipeline>, RenderManagerError> {
        let blit_vert_shader =
            create_shader_module(device.clone(), "assets/shader_binaries/blit.vert.spv")?;
        let blit_frag_shader =
            create_shader_module(device.clone(), "assets/shader_binaries/blit.frag.spv")?;

        GraphicsPipeline::start()
            .render_pass(PipelineRenderingCreateInfo {
                color_attachment_formats: vec![Some(swapchain_image_format)],
                ..Default::default()
            })
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .vertex_shader(
                blit_vert_shader
                    .entry_point("main")
                    .unrec_err("no main in blit.vert")?,
                (),
            )
            .fragment_shader(
                blit_frag_shader
                    .entry_point("main")
                    .unrec_err("no main in blit.frag")?,
                (),
            )
            .build(device.clone())
            .unrec_err("failed to create blit graphics pipeline")
    }

    fn create_blit_desc_set(
        blit_pipeline: Arc<GraphicsPipeline>,
        render_image: Arc<ImageView<StorageImage>>,
        render_image_sampler: Arc<Sampler>,
    ) -> Result<Arc<PersistentDescriptorSet>, RenderManagerError> {
        PersistentDescriptorSet::new(
            blit_pipeline
                .layout()
                .set_layouts()
                .get(shader_interfaces::descriptor::SET_BLIT_FRAG)
                .unwrap()
                .to_owned(),
            [WriteDescriptorSet::image_view_sampler(
                shader_interfaces::descriptor::BINDING_SAMPLER,
                render_image,
                render_image_sampler,
            )],
        )
        .unrec_err("unable to create blit pass descriptor set")
    }

    /// Recreates the swapchain, render image and assiciated descriptor sets, then unsets `recreate_swapchain` trigger.
    fn recreate_swapchain(&mut self) -> Result<(), RenderManagerError> {
        debug!("recreating swapchain and render targets...");

        let (new_swapchain, swapchain_images) =
            match self.swapchain.recreate(swapchain::SwapchainCreateInfo {
                image_extent: self.surface.window().inner_size().into(),
                ..self.swapchain.create_info()
            }) {
                Ok(r) => r,
                Err(e) => return Err(e.into()),
            };

        self.swapchain = new_swapchain;
        self.swapchain_image_views = swapchain_images
            .iter()
            .map(|image| ImageView::new_default(image.clone()).unwrap())
            .collect::<Vec<_>>();

        // set parameters for new resolution
        let resolution = swapchain_images[0].dimensions().width_height();
        self.work_group_count = calc_work_group_count(
            self.device.physical_device(),
            resolution,
            self.work_group_size,
        )?;
        self.viewport.dimensions = [resolution[0] as f32, resolution[1] as f32];

        self.render_image = Self::create_render_image(
            self.queue.clone(),
            swapchain_images[0].dimensions().width_height(),
        )?;
        self.render_desc_set =
            Self::create_compute_desc_set(self.render_pipeline.clone(), self.render_image.clone())?;
        self.blit_desc_set = Self::create_blit_desc_set(
            self.blit_pipeline.clone(),
            self.render_image.clone(),
            self.render_image_sampler.clone(),
        )?;

        // unset trigger
        self.recreate_swapchain = false;

        Ok(())
    }
}

// ~~~ Errors ~~~

/// Describes the types of errors encountered by the renderer
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderManagerError {
    /// An unrecoverable or unexpected error has prevented the RenderManager from initializing or rendering.
    /// Contains an string explaining the cause.
    Unrecoverable(String),

    /// The window surface is no longer accessible and must be recreated.
    /// Invalidates the RenderManger and requires re-initialization.
    ///
    /// Equivalent to vulkano [SurfacePropertiesError::SurfaceLost](`vulkano::device::physical::SurfacePropertiesError::SurfaceLost`)
    SurfaceLost,

    /// Requested dimensions are not within supported range when attempting to create a render target (swapchain)
    /// This error tends to happen when the user is manually resizing the window.
    /// Simply restarting the loop is the easiest way to fix this issue.
    ///
    /// Equivalent to vulkano [SwapchainCreationError::ImageExtentNotSupported](`vulkano::swapchain::SwapchainCreationError::ImageExtentNotSupported`)
    SurfaceSizeUnsupported {
        provided: [u32; 2],
        min_supported: [u32; 2],
        max_supported: [u32; 2],
    },
}
impl error::Error for RenderManagerError {}
impl fmt::Display for RenderManagerError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            RenderManagerError::Unrecoverable(msg) => write!(fmt, "{}", msg),
            RenderManagerError::SurfaceLost =>
                write!(fmt, "the Vulkan surface is no longer accessible, thus invalidating this RenderManager instance"),
            RenderManagerError::SurfaceSizeUnsupported{ provided, min_supported, max_supported } =>
                write!(fmt, "cannot create render target with requested dimensions = {:?}. min size = {:?}, max size = {:?}",
                    provided, min_supported, max_supported),
        }
    }
}
impl RenderManagerError {
    /// Passes the error through the `error!` log and returns self
    #[inline]
    pub fn log(self) -> Self {
        error!("{:?}", self);
        self
    }
}
trait RenderManagerUnrecoverable<T> {
    /// Shorthand for converting a general error to a RenderManagerError::InitFailed.
    /// Commonly used with error propogation `?` in RenderManager::new.
    fn unrec_err(self, msg: &str) -> Result<T, RenderManagerError>;
}
impl<T, E> RenderManagerUnrecoverable<T> for std::result::Result<T, E>
where
    E: fmt::Debug,
{
    #[inline]
    #[track_caller]
    fn unrec_err(self, msg: &str) -> Result<T, RenderManagerError> {
        match self {
            Ok(x) => Ok(x),
            Err(e) => Err(RenderManagerError::Unrecoverable(format!("{}: {:?}", msg, e)).log()),
        }
    }
}
impl<T> RenderManagerUnrecoverable<T> for std::option::Option<T> {
    #[inline]
    #[track_caller]
    fn unrec_err(self, msg: &str) -> Result<T, RenderManagerError> {
        match self {
            Some(x) => Ok(x),
            None => Err(RenderManagerError::Unrecoverable(msg.to_owned()).log()),
        }
    }
}
impl From<SwapchainCreationError> for RenderManagerError {
    fn from(error: SwapchainCreationError) -> Self {
        use RenderManagerError::{SurfaceSizeUnsupported, Unrecoverable};
        match error {
            // this error tends to happen when the user is manually resizing the window.
            // simply restarting the loop is the easiest way to fix this issue.
            SwapchainCreationError::ImageExtentNotSupported {
                provided,
                min_supported,
                max_supported,
            } => {
                let err = SurfaceSizeUnsupported {
                    provided,
                    min_supported,
                    max_supported,
                };
                debug!("cannot create swapchain: {:?}", err);
                err
            }
            e => Unrecoverable(format!("Failed to recreate swapchain: {:?}", e)).log(),
        }
    }
}

/// Describes issues with enabling instance extensions/layers
#[derive(Clone, Debug)]
enum InstanceSupportError {
    /// Requested instance extension is not supported by this vulkan driver
    ExtensionUnsupported,
    /// Requested Vulkan layer is not found (may not be installed)
    LayerNotFound,
    /// Failed to load the Vulkan shared library.
    LayersListError(instance::LayersListError),
}
impl From<instance::LayersListError> for InstanceSupportError {
    #[inline]
    fn from(err: LayersListError) -> Self {
        Self::LayersListError(err)
    }
}

// ~~~ Helper functions ~~~

/// Creates a Vulkan shader module given a spirv path (relative to crate root)
pub fn create_shader_module(
    device: Arc<Device>,
    spirv_path: &str,
) -> Result<Arc<ShaderModule>, RenderManagerError> {
    let bytes = std::fs::read(spirv_path).unrec_err("render.comp.spv read failed")?;
    // todo conv to &[u32] and use from_words (guarentees 4 byte multiple)
    unsafe { ShaderModule::from_bytes(device.clone(), bytes.as_slice()) }
        .unrec_err([spirv_path, "shader compile failed"].concat().as_str())
}

/// Checks for VK_EXT_debug_utils support and presence khronos validation layers
/// If both can be enabled, adds them to provided extension and layer lists
fn add_debug_validation(
    instance_extensions: &mut instance::InstanceExtensions,
    instance_layers: &mut Vec<String>,
) -> Result<(), InstanceSupportError> {
    // check debug utils extension support
    if match instance::InstanceExtensions::supported_by_core() {
        Ok(supported) => supported.ext_debug_utils,
        Err(_) => false,
    } {
        info!("VK_EXT_debug_utils was requested and is supported");
    } else {
        warn!("VK_EXT_debug_utils was requested but is unsupported");
        return Err(InstanceSupportError::ExtensionUnsupported);
    }

    // check validation layers are present
    let validation_layer = "VK_LAYER_KHRONOS_validation";
    if instance::layers_list()?
        .find(|l| l.name() == validation_layer)
        .is_some()
    {
        info!("{} was requested and found", validation_layer);
    } else {
        warn!("{} was requested but was not found", validation_layer);
        return Err(InstanceSupportError::LayerNotFound);
    }

    // add VK_EXT_debug_utils and VK_LAYER_LUNARG_standard_validation
    instance_extensions.ext_debug_utils = true;
    instance_layers.push(validation_layer.to_owned());
    Ok(())
}

/// This mod just makes the module path unique for debug callbacks in the log
mod vulkan_callback {
    use colored::Colorize;
    use log::{debug, error, info, warn};
    use vulkano::instance::debug::Message;
    /// Prints/logs a Vulkan validation layer message
    pub fn process_debug_callback(msg: &Message) {
        let ty = if msg.ty.general {
            "GENERAL"
        } else if msg.ty.validation {
            "VALIDATION"
        } else if msg.ty.performance {
            "PERFORMANCE"
        } else {
            "TYPE-UNKNOWN"
        };
        if msg.severity.error {
            error!("Vulkan [{}]:\n{}", ty, msg.description.bright_red());
        } else if msg.severity.warning {
            warn!("Vulkan [{}]:\n{}", ty, msg.description);
        } else if msg.severity.information {
            info!("Vulkan [{}]:\n{}", ty, msg.description);
        } else if msg.severity.verbose {
            debug!("Vulkan [{}]:\n{}", ty, msg.description);
        } else {
            info!(
                "Vulkan [{}] [{}]:\n{}",
                "SEVERITY-UNKONWN", ty, msg.description
            );
        };
    }
}

/// Calculate required work group count for a given render resolution,
/// and checks that the work group count is within the physical device limits
pub fn calc_work_group_count(
    physical_device: PhysicalDevice,
    resolution: [u32; 2],
    work_group_size: [u32; 2],
) -> Result<[u32; 3], RenderManagerError> {
    let mut group_count_x = resolution[0] / work_group_size[0];
    if (resolution[0] % work_group_size[0]) != 0 {
        group_count_x += 1;
    }
    let mut group_count_y = resolution[1] / work_group_size[1];
    if (resolution[1] % work_group_size[1]) != 0 {
        group_count_y += 1;
    }
    // check that work group count is within physical device limits
    // todo this can be handled more elegently by doing multiple dispatches...
    if group_count_x > physical_device.properties().max_compute_work_group_count[0]
        || group_count_y > physical_device.properties().max_compute_work_group_count[1]
    {
        return Err(RenderManagerError::Unrecoverable(
            "compute shader work group count exceeds physical device limits. TODO this can be handled more elegently by doing multiple dispatches...".to_string(),
        ));
    }
    Ok([group_count_x, group_count_y, 1])
}
