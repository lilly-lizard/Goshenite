use crate::camera::Camera;
use crate::config;
use crate::shaders::shader_interfaces;
use log::{debug, error, info, warn};
use std::{error, fmt, sync::Arc};
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

/// Describes the types of errors encountered by the renderer
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderManagerError {
    /// An unrecoverable or unexpected error has prevented the RenderManager from initializing or rendering.
    /// Contains an string explaining the cause.
    Unrecoverable(String),

    /// The window surface is no longer accessible and must be recreated.
    /// Invalidates the RenderManger and requires re-initialization.
    ///
    /// Equivalent to [`vulkano::device::physical::SurfacePropertiesError::SurfaceLost`]
    SurfaceLost,

    /// Requested dimensions are not within supported range when attempting to create a render target (swapchain)
    /// This error tends to happen when the user is manually resizing the window.
    /// Simply restarting the loop is the easiest way to fix this issue.
    ///
    /// Equivalent to [`vulkano::swapchain::SwapchainCreationError::ImageExtentNotSupported`]
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

/// Contains Vulkan resources and methods to manage rendering
pub struct RenderManager {
    _debug_callback: Option<DebugUtilsMessenger>,
    pub device: Arc<Device>,
    pub queue: Arc<device::Queue>,

    surface: Arc<swapchain::Surface<Arc<Window>>>,
    swapchain: Arc<swapchain::Swapchain<Arc<Window>>>,
    swapchain_image_views: Vec<Arc<ImageView<SwapchainImage<Arc<Window>>>>>,

    render_image: Arc<ImageView<StorageImage>>,
    render_image_format: Format,
    render_image_sampler: Arc<sampler::Sampler>,

    render_pipeline: Arc<pipeline::ComputePipeline>,
    render_desc_set: Arc<PersistentDescriptorSet>,
    work_group_size: [u32; 2],
    work_group_count: [u32; 3],
    blit_pipeline: Arc<pipeline::GraphicsPipeline>,
    blit_desc_set: Arc<PersistentDescriptorSet>,
    viewport: Viewport,

    future_previous_frame: Option<Box<dyn vulkano::sync::GpuFuture>>, // todo description
    /// indicates that the swapchain needs to be recreated next frame
    recreate_swapchain: bool,
}
// Public functions
impl RenderManager {
    /// Initializes Vulkan resources. If renderer fails to initialize, returns a string explanation.
    pub fn new(window: Arc<Window>) -> Result<Self, RenderManagerError> {
        use RenderManagerError::{SurfaceSizeUnsupported, Unrecoverable};

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
        let instance = instance::Instance::new(instance::InstanceCreateInfo {
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

        let surface = vulkano_win::create_surface_from_winit(window, instance.clone())
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
        let (swapchain, swapchain_images) = {
            // todo prefer sRGB (linux sRGB)
            let image_format =
                match physical_device.surface_formats(&surface, Default::default()) {
                    Ok(x) => x,
                    Err(SurfacePropertiesError::SurfaceLost) => {
                        return Err(RenderManagerError::SurfaceLost.log())
                    }
                    Err(e) => {
                        return Err(
                            Unrecoverable(format!("failed to get surface format: {:?}", e)).log(),
                        )
                    }
                }
                .get(0)
                .expect("vulkan driver should support at least 1 surface format... right?")
                .0;
            debug!("swapchain image format = {:?}", image_format);

            let surface_capabilities =
                match physical_device.surface_capabilities(&surface, Default::default()) {
                    Ok(x) => x,
                    Err(SurfacePropertiesError::SurfaceLost) => {
                        return Err(RenderManagerError::SurfaceLost.log())
                    }
                    Err(e) => {
                        return Err(Unrecoverable(format!(
                            "failed to get surface capabilities: {:?}",
                            e,
                        ))
                        .log())
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
                    return Err(Unrecoverable(format!(
                        "failed to get surface capabilities: {:?}",
                        e,
                    ))
                    .log())
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
                Ok(x) => x,
                Err(swapchain::SwapchainCreationError::ImageExtentNotSupported {
                    provided,
                    min_supported,
                    max_supported,
                }) => {
                    let err = SurfaceSizeUnsupported {
                        provided,
                        min_supported,
                        max_supported,
                    };
                    warn!("cannot create swapchain: {:?}", err);
                    return Err(err);
                }
                Err(e) => {
                    return Err(Unrecoverable(format!("failed to create swapchain: {:?}", e)).log())
                }
            }
        };
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

        // create swapchain image views
        let swapchain_image_views: Result<Vec<_>, _> = swapchain_images
            .iter()
            .map(|image| ImageView::new_default(image.clone()))
            .collect();
        let swapchain_image_views =
            swapchain_image_views.unrec_err("failed to create swapchain image view(s)")?;

        // compute shader render target
        let render_image_format = Format::R8G8B8A8_UNORM;
        let render_image = StorageImage::general_purpose_image_view(
            queue.clone(),
            swapchain_images[0].dimensions().width_height(),
            render_image_format,
            ImageUsage {
                storage: true,
                sampled: true,
                ..ImageUsage::none()
            },
        )
        .unrec_err("failed to create render image")?;

        let render_image_sampler = sampler::Sampler::new(
            device.clone(),
            sampler::SamplerCreateInfo {
                mag_filter: sampler::Filter::Linear,
                min_filter: sampler::Filter::Linear,
                address_mode: [sampler::SamplerAddressMode::Repeat; 3],
                ..Default::default()
            },
        )
        .unrec_err("failed to create sampler")?;

        let shader_render =
            create_shader_module(device.clone(), "assets/shader_binaries/render.comp.spv")?;
        let shader_blit_vert =
            create_shader_module(device.clone(), "assets/shader_binaries/blit.vert.spv")?;
        let shader_blit_frag =
            create_shader_module(device.clone(), "assets/shader_binaries/blit.frag.spv")?;

        let render_pipeline = pipeline::ComputePipeline::new(
            device.clone(),
            shader_render
                .entry_point("main")
                .unrec_err("no main in render.comp")?,
            &(),
            None,
            |_| {},
        )
        .unwrap();

        let work_group_size = config::DEFAULT_WORK_GROUP_SIZE;
        let work_group_count = calc_work_group_count(
            swapchain_images[0].dimensions().width_height(),
            work_group_size,
        );

        let blit_pipeline = pipeline::GraphicsPipeline::start()
            .render_pass(
                pipeline::graphics::render_pass::PipelineRenderingCreateInfo {
                    color_attachment_formats: vec![Some(swapchain.image_format())],
                    ..Default::default()
                },
            )
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .vertex_shader(
                shader_blit_vert
                    .entry_point("main")
                    .unrec_err("no main in blit.vert")?,
                (),
            )
            .fragment_shader(
                shader_blit_frag
                    .entry_point("main")
                    .unrec_err("no main in blit.frag")?,
                (),
            )
            .build(device.clone())
            .unwrap();

        let render_desc_set = PersistentDescriptorSet::new(
            render_pipeline
                .layout()
                .set_layouts()
                .get(shader_interfaces::descriptor::SET_RENDER_COMP)
                .unwrap()
                .to_owned(),
            [WriteDescriptorSet::image_view(
                shader_interfaces::descriptor::BINDING_IMAGE,
                render_image.clone(),
            )],
        )
        .unwrap();

        let blit_desc_set = PersistentDescriptorSet::new(
            blit_pipeline
                .layout()
                .set_layouts()
                .get(shader_interfaces::descriptor::SET_BLIT_FRAG)
                .unwrap()
                .to_owned(),
            [WriteDescriptorSet::image_view_sampler(
                shader_interfaces::descriptor::BINDING_SAMPLER,
                render_image.clone(),
                render_image_sampler.clone(),
            )],
        )
        .unwrap();

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
            render_image_format,
            render_image_sampler,
            render_pipeline,
            blit_pipeline,
            render_desc_set,
            blit_desc_set,
            work_group_size,
            work_group_count,
            future_previous_frame,
            recreate_swapchain,
        })
    }

    /// Submits Vulkan commands for rendering a frame.
    pub fn render_frame(
        &mut self,
        window_resize: bool,
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
            .set_viewport(0, [self.viewport.clone()])
            .bind_pipeline_graphics(self.blit_pipeline.clone())
            .bind_descriptor_sets(
                pipeline::PipelineBindPoint::Graphics,
                self.blit_pipeline.layout().clone(),
                0,
                self.blit_desc_set.clone(),
            )
            .draw(3, 1, 0, 0)
            .unwrap()
            .end_rendering()
            .unwrap();
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
    /// Recreates the swapchain, render image and assiciated descriptor sets. Unsets `recreate_swapchain` trigger
    fn recreate_swapchain(&mut self) -> Result<(), RenderManagerError> {
        use RenderManagerError::{SurfaceSizeUnsupported, Unrecoverable};
        debug!("recreating swapchain and render targets...");

        let (new_swapchain, swapchain_images) =
            match self.swapchain.recreate(swapchain::SwapchainCreateInfo {
                image_extent: self.surface.window().inner_size().into(),
                ..self.swapchain.create_info()
            }) {
                Ok(r) => r,
                // this error tends to happen when the user is manually resizing the window.
                // simply restarting the loop is the easiest way to fix this issue.
                Err(swapchain::SwapchainCreationError::ImageExtentNotSupported {
                    provided,
                    min_supported,
                    max_supported,
                }) => {
                    let err = SurfaceSizeUnsupported {
                        provided,
                        min_supported,
                        max_supported,
                    };
                    debug!("cannot recreate swapchain: {:?}", err);
                    return Err(err);
                }
                Err(e) => {
                    return Err(
                        Unrecoverable(format!("Failed to recreate swapchain: {:?}", e)).log(),
                    );
                }
            };

        self.swapchain = new_swapchain;
        self.swapchain_image_views = swapchain_images
            .iter()
            .map(|image| ImageView::new_default(image.clone()).unwrap())
            .collect::<Vec<_>>();

        // set parameters for new resolution
        let resolution = swapchain_images[0].dimensions().width_height();
        self.work_group_count = calc_work_group_count(resolution, self.work_group_size);
        self.viewport.dimensions = [resolution[0] as f32, resolution[1] as f32];

        // compute shader render target
        self.render_image = StorageImage::general_purpose_image_view(
            self.queue.clone(),
            resolution,
            self.render_image_format,
            ImageUsage {
                storage: true,
                sampled: true,
                ..ImageUsage::none()
            },
        )
        .unwrap();

        self.render_desc_set = PersistentDescriptorSet::new(
            self.render_pipeline
                .layout()
                .set_layouts()
                .get(shader_interfaces::descriptor::SET_RENDER_COMP)
                .unwrap()
                .to_owned(),
            [WriteDescriptorSet::image_view(
                shader_interfaces::descriptor::BINDING_IMAGE,
                self.render_image.clone(),
            )],
        )
        .unwrap();

        self.blit_desc_set = PersistentDescriptorSet::new(
            self.blit_pipeline
                .layout()
                .set_layouts()
                .get(shader_interfaces::descriptor::SET_BLIT_FRAG)
                .unwrap()
                .to_owned(),
            [WriteDescriptorSet::image_view_sampler(
                shader_interfaces::descriptor::BINDING_SAMPLER,
                self.render_image.clone(),
                self.render_image_sampler.clone(),
            )],
        )
        .unwrap();

        // unset trigger
        self.recreate_swapchain = false;

        Ok(())
    }
}

// Helper functions

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

/// Describes issues with enabling instance extensions/layers
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

/// Calculate required work group count for a given render resolution
pub fn calc_work_group_count(resolution: [u32; 2], work_group_size: [u32; 2]) -> [u32; 3] {
    let mut group_count_x = resolution[0] / work_group_size[0];
    if (resolution[0] % work_group_size[0]) != 0 {
        group_count_x += 1;
    }
    let mut group_count_y = resolution[1] / work_group_size[1];
    if (resolution[1] % work_group_size[1]) != 0 {
        group_count_y += 1;
    }
    [group_count_x, group_count_y, 1]
}
