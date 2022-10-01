use super::blit_pass::BlitPass;
use super::gui_renderer::GuiRenderer;
use super::scene_pass::ScenePass;
use crate::config;
use crate::gui::Gui;
use crate::primitives::primitives::PrimitiveCollection;
use crate::shaders::shader_interfaces::CameraPushConstant;
use crate::{camera::Camera, helper::from_err_impl::from_err_impl};
use log::{debug, error, info, warn};
use std::{error, fmt, sync::Arc};
use vulkano::device::QueueCreateInfo;
use vulkano::swapchain::PresentInfo;
use vulkano::{
    command_buffer,
    device::{self, Device, Queue},
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        DeviceExtensions,
    },
    format::Format,
    image::{view::ImageView, ImageAccess, ImageUsage, StorageImage, SwapchainImage},
    instance::debug::{
        DebugUtilsMessageSeverity, DebugUtilsMessageType, DebugUtilsMessenger,
        DebugUtilsMessengerCreateInfo,
    },
    instance::{self, Instance},
    pipeline::graphics::viewport::Viewport,
    render_pass::{LoadOp, StoreOp},
    swapchain::{self, Surface, Swapchain, SwapchainCreationError},
    sync::{self, FlushError, GpuFuture},
};
use vulkano::{OomError, VulkanLibrary};
use winit::window::Window;

/// Contains Vulkan resources and methods to manage rendering
pub struct RenderManager {
    device: Arc<Device>,
    render_queue: Arc<Queue>,
    _transfer_queue: Arc<Queue>,
    _debug_callback: Option<DebugUtilsMessenger>,

    surface: Arc<Surface<Arc<Window>>>,
    swapchain: Arc<Swapchain<Arc<Window>>>,
    swapchain_image_views: Vec<Arc<ImageView<SwapchainImage<Arc<Window>>>>>,
    viewport: Viewport,
    render_image: Arc<ImageView<StorageImage>>,

    scene_pass: ScenePass,
    blit_pass: BlitPass,
    gui_pass: GuiRenderer,

    future_previous_frame: Option<Box<dyn GpuFuture>>, // todo description
    /// indicates that the swapchain needs to be recreated next frame
    recreate_swapchain: bool,
}

/// Indicates a queue family index
pub type QueueFamilyIndex = u32;

// ~~~ Public functions ~~~

impl RenderManager {
    /// Initializes Vulkan resources. If renderer fails to initialize, returns a string explanation.
    pub fn new(
        window: Arc<Window>,
        primitives: &PrimitiveCollection,
    ) -> Result<Self, RenderManagerError> {
        // load vulkan library
        let vulkan_library =
            VulkanLibrary::new().to_renderer_err("failed to load vulkan library")?;

        // required instance extensions for platform surface rendering
        let mut instance_extensions = vulkano_win::required_extensions(&vulkan_library);
        let mut instance_layers: Vec<String> = Vec::new();

        // check for validation layer/debug callback support
        let enable_debug_callback = if config::ENABLE_VULKAN_VALIDATION {
            if Self::add_debug_validation(
                vulkan_library.clone(),
                &mut instance_extensions,
                &mut instance_layers,
            )
            .is_ok()
            {
                info!("enabling Vulkan validation layers and debug callback");
                true
            } else {
                warn!("validation layer debug callback requested but cannot be enabled");
                false
            }
        } else {
            debug!("Vulkan validation layers disabled by config");
            false
        };

        // create instance
        let instance = Instance::new(
            vulkan_library.clone(),
            instance::InstanceCreateInfo {
                enabled_extensions: instance_extensions,
                enumerate_portability: true, // enable enumerating devices that use non-conformant vulkan implementations. (ex. MoltenVK)
                enabled_layers: instance_layers,
                ..Default::default()
            },
        )
        .to_renderer_err("Failed to create vulkan instance")?;

        // setup debug callback
        let debug_callback = if enable_debug_callback {
            Self::setup_debug_callback(instance.clone())
        } else {
            None
        };

        // create surface
        let surface = vulkano_win::create_surface_from_winit(window.clone(), instance.clone())
            .to_renderer_err("failed to create vulkan surface")?;

        // required device extensions
        let device_extensions = device::DeviceExtensions {
            khr_swapchain: true,
            ..device::DeviceExtensions::empty()
        };

        // print available physical devices
        debug!("Available Vulkan physical devices:");
        for pd in instance
            .enumerate_physical_devices()
            .to_renderer_err("failed to enumerate physical devices")?
        {
            debug!("\t{}", pd.properties().device_name);
        }
        // choose physical device and queue families
        let ChoosePhysicalDeviceReturn {
            physical_device,
            render_queue_family,
            transfer_queue_family,
        } = Self::choose_physical_device(instance.clone(), &device_extensions, &surface)?;
        info!(
            "Using Vulkan device: {} (type: {:?})",
            physical_device.properties().device_name,
            physical_device.properties().device_type,
        );

        // queue create info(s) for creating render and transfer queues
        let single_queue = (render_queue_family == transfer_queue_family)
            && (physical_device.queue_family_properties()[render_queue_family as usize]
                .queue_count
                == 1);
        let queue_create_infos = if render_queue_family == transfer_queue_family {
            vec![QueueCreateInfo {
                queue_family_index: render_queue_family,
                queues: if single_queue {
                    vec![0.5]
                } else {
                    vec![0.5; 2]
                },
                ..Default::default()
            }]
        } else {
            vec![
                QueueCreateInfo {
                    queue_family_index: render_queue_family,
                    ..Default::default()
                },
                QueueCreateInfo {
                    queue_family_index: transfer_queue_family,
                    ..Default::default()
                },
            ]
        };
        // create device and queues
        let (device, mut queues) = device::Device::new(
            physical_device.clone(),
            device::DeviceCreateInfo {
                enabled_extensions: device_extensions,
                enabled_features: device::Features {
                    dynamic_rendering: true,
                    ..device::Features::empty()
                },
                queue_create_infos,
                ..Default::default()
            },
        )
        .to_renderer_err("failed to create vulkan device and queues")?;
        let render_queue = queues
            .next()
            .expect("requested 1 queue from render_queue_family");
        let transfer_queue = if single_queue {
            render_queue.clone()
        } else {
            queues.next().expect("requested 1 unique transfer queue")
        };

        // create swapchain and images
        let (swapchain, swapchain_images) =
            Self::create_swapchain(device.clone(), physical_device.clone(), surface.clone())?;
        debug!(
            "initial swapchain image size = {:?}",
            swapchain_images[0].dimensions()
        );

        // init dynamic viewport
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
            swapchain_image_views.to_renderer_err("failed to create swapchain image view(s)")?;

        // scene render target
        let render_image = Self::create_render_image(
            render_queue.clone(),
            swapchain_images[0].dimensions().width_height(),
        )?;

        // init compute shader scene pass
        let scene_pass = ScenePass::new(
            device.clone(),
            primitives,
            swapchain_images[0].dimensions().width_height(),
            render_image.clone(),
        )
        .to_renderer_err("failed to initialize scene pass")?;

        // init blit pass
        let blit_pass = BlitPass::new(
            device.clone(),
            swapchain.image_format(),
            render_image.clone(),
        )
        .to_renderer_err("failed to initialize blit pass")?;

        // init gui renderer
        let gui_pass = GuiRenderer::new(
            device.clone(),
            transfer_queue.clone(),
            swapchain.image_format(),
        )
        .to_renderer_err("failed to initialize gui pass")?;

        // create futures used for frame synchronization
        let future_previous_frame = Some(sync::now(device.clone()).boxed());
        let recreate_swapchain = false;

        Ok(RenderManager {
            _debug_callback: debug_callback,
            device,
            render_queue,
            _transfer_queue: transfer_queue,
            surface,
            swapchain,
            swapchain_image_views,
            viewport,
            render_image,
            scene_pass,
            blit_pass,
            gui_pass,
            future_previous_frame,
            recreate_swapchain,
        })
    }

    /// Returns a mutable reference to the gui renderer so its resources can be updated by the gui
    pub fn gui_renderer(&mut self) -> &mut GuiRenderer {
        &mut self.gui_pass
    }

    /// Submits Vulkan commands for rendering a frame.
    pub fn render_frame(
        &mut self,
        window_resize: bool,
        primitives: &PrimitiveCollection,
        gui: &Gui,
        camera: Camera,
    ) -> Result<(), RenderManagerError> {
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
        let (image_index, suboptimal, acquire_future) =
            match swapchain::acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => r,
                Err(swapchain::AcquireError::OutOfDate) => {
                    self.recreate_swapchain = true;
                    // recreate swapchain and skip frame render
                    return self.recreate_swapchain();
                }
                Err(e) => {
                    // todo other error handling
                    return Err(RenderManagerError::Unrecoverable {
                        message: "Failed to acquire next image".to_owned(),
                        source: Some(e.into()),
                    });
                }
            };
        if suboptimal {
            self.recreate_swapchain = true;
        }

        // todo shouldn't need to recreate each frame??
        self.scene_pass
            .update_primitives(primitives)
            .to_renderer_err("failed to update primitives")?;

        let need_srgb_conv = false; // todo

        // record command buffer
        let mut builder = command_buffer::AutoCommandBufferBuilder::primary(
            self.device.clone(),
            self.render_queue.queue_family_index(),
            command_buffer::CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();
        // compute shader scene render
        let camera_push_constant = CameraPushConstant::new(
            glam::Mat4::inverse(&(camera.proj_matrix() * camera.view_matrix())),
            camera.position(),
        );
        self.scene_pass
            .record_commands(&mut builder, camera_push_constant)
            .to_renderer_err("failed to dispatch compute shader")?;
        // begin render pass
        builder
            .begin_rendering(command_buffer::RenderingInfo {
                color_attachments: vec![Some(command_buffer::RenderingAttachmentInfo {
                    load_op: LoadOp::Clear,
                    store_op: StoreOp::Store,
                    clear_value: Some([0.0, 1.0, 0.0, 1.0].into()),
                    ..command_buffer::RenderingAttachmentInfo::image_view(
                        self.swapchain_image_views[image_index as usize].clone(),
                    )
                })],
                ..Default::default()
            })
            .to_renderer_err("failed to record vkCmdBeginRendering")?;
        // draw render image to screen
        self.blit_pass
            .record_commands(&mut builder, self.viewport.clone())
            .to_renderer_err("failed to record blit pass draw commands")?;
        // render gui todo return error
        self.gui_pass
            .record_commands(
                &mut builder,
                gui,
                need_srgb_conv,
                [
                    self.viewport.dimensions[0] as u32,
                    self.viewport.dimensions[1] as u32,
                ],
            )
            .to_renderer_err("failed to record gui commands")?;
        // end render pass
        builder
            .end_rendering()
            .to_renderer_err("failed to record vkCmdEndRendering")?;
        let command_buffer = builder
            .build()
            .to_renderer_err("failed to build command buffer")?;

        // submit
        let future = self
            .future_previous_frame
            .take()
            .unwrap()
            .join(acquire_future)
            .then_execute(self.render_queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(
                self.render_queue.clone(),
                PresentInfo {
                    index: image_index,
                    ..PresentInfo::swapchain(self.swapchain.clone())
                },
            )
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
                error!("Failed to flush future: {}", e);
                self.future_previous_frame = Some(sync::now(self.device.clone()).boxed());
            }
        }
        Ok(())
    }
}

// ~~~ Private functions ~~~

impl RenderManager {
    /// Checks for VK_EXT_debug_utils support and presence khronos validation layers
    /// If both can be enabled, adds them to provided extension and layer lists
    fn add_debug_validation(
        vulkan_library: Arc<VulkanLibrary>,
        instance_extensions: &mut instance::InstanceExtensions,
        instance_layers: &mut Vec<String>,
    ) -> Result<(), InstanceSupportError> {
        // check debug utils extension support
        if vulkan_library.supported_extensions().ext_debug_utils {
            info!("VK_EXT_debug_utils was requested and is supported");
        } else {
            warn!("VK_EXT_debug_utils was requested but is unsupported");
            return Err(InstanceSupportError::ExtensionUnsupported {
                extension: "VK_EXT_debug_utils",
            });
        }

        // check validation layers are present
        let validation_layer = "VK_LAYER_KHRONOS_validation";
        if vulkan_library
            .layer_properties()?
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

    fn setup_debug_callback(instance: Arc<Instance>) -> Option<DebugUtilsMessenger> {
        unsafe {
            match DebugUtilsMessenger::new(
                instance,
                DebugUtilsMessengerCreateInfo {
                    message_severity: DebugUtilsMessageSeverity {
                        error: true,
                        warning: true,
                        information: true,
                        verbose: false,
                        ..DebugUtilsMessageSeverity::empty()
                    },
                    message_type: DebugUtilsMessageType {
                        general: true,
                        validation: true,
                        performance: true,
                        ..DebugUtilsMessageType::empty()
                    },
                    ..DebugUtilsMessengerCreateInfo::user_callback(Arc::new(|msg| {
                        vulkan_callback::process_debug_callback(msg)
                    }))
                },
            ) {
                Ok(x) => Some(x),
                Err(e) => {
                    warn!("failed to setup vulkan debug callback: {}", e,);
                    None
                }
            }
        }
    }

    fn choose_physical_device(
        instance: Arc<Instance>,
        device_extensions: &DeviceExtensions,
        surface: &Arc<Surface<Arc<Window>>>,
    ) -> Result<ChoosePhysicalDeviceReturn, RenderManagerError> {
        instance
            .enumerate_physical_devices()
            .to_renderer_err("failed to enumerate physical devices")?
            // filter for vulkan version support
            .filter(|p| {
                p.api_version()
                    >= vulkano::Version::major_minor(config::VULKAN_VER_MAJ, config::VULKAN_VER_MIN)
            })
            // filter for required device extensions
            .filter(|p| p.supported_extensions().contains(device_extensions))
            // filter for queue support
            .filter_map(|p| {
                // get queue family index for main queue used for rendering
                let render_family = p
                    .queue_family_properties()
                    .iter()
                    // because we want the queue family index
                    .enumerate()
                    .position(|(i, q)| {
                        // must support our surface and essential operations
                        q.queue_flags.graphics
                            && q.queue_flags.compute
                            && q.queue_flags.transfer
                            && p.surface_support(i as u32, surface).unwrap_or(false)
                    });
                if let Some(render_index) = render_family {
                    // attempt to find a different queue family that we can use for asynchronous transfer operations
                    // e.g. uploading image/buffer data while rendering
                    let transfer_family = p
                        .queue_family_properties()
                        .iter()
                        // because we want the queue family index
                        .enumerate()
                        // exclude the queue family we've already found and filter by transfer operation support
                        .filter(|(i, q)| *i != render_index && q.queue_flags.transfer)
                        // some drivers expose a queue that only supports transfer operations (for this very purpose) which is preferable
                        .max_by_key(|(_, q)| {
                            if !q.queue_flags.compute && !q.queue_flags.graphics {
                                1
                            } else {
                                0
                            }
                        })
                        .map(|(i, _)| i);
                    Some(ChoosePhysicalDeviceReturn {
                        physical_device: p,
                        render_queue_family: render_index as QueueFamilyIndex,
                        transfer_queue_family: transfer_family.unwrap_or(render_index)
                            as QueueFamilyIndex,
                    })
                } else {
                    // failed to find main queue
                    todo!("anyhow error context messages...");
                    None
                }
            })
            // preference of device type
            .max_by_key(
                |ChoosePhysicalDeviceReturn {
                     physical_device, ..
                 }| match physical_device.properties().device_type {
                    PhysicalDeviceType::DiscreteGpu => 4,
                    PhysicalDeviceType::IntegratedGpu => 3,
                    PhysicalDeviceType::VirtualGpu => 2,
                    PhysicalDeviceType::Cpu => 1,
                    PhysicalDeviceType::Other => 0,
                    _ne => 0,
                },
            )
            .to_renderer_err("no suitable physical device available")
    }

    fn create_swapchain(
        device: Arc<Device>,
        physical_device: Arc<PhysicalDevice>,
        surface: Arc<Surface<Arc<Window>>>,
    ) -> Result<
        (
            Arc<Swapchain<Arc<Window>>>,
            Vec<Arc<SwapchainImage<Arc<Window>>>>,
        ),
        RenderManagerError,
    > {
        // todo prefer sRGB (linux sRGB)
        let image_format = physical_device
            .surface_formats(&surface, Default::default())
            .to_renderer_err("failed to get surface formats")?
            .get(0)
            .expect("vulkan driver should support at least 1 surface format... right?")
            .0;
        debug!("swapchain image format = {:?}", image_format);

        let surface_capabilities = physical_device
            .surface_capabilities(&surface, Default::default())
            .to_renderer_err("failed to get surface capabilities")?;
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

        let mut present_modes = physical_device
            .surface_present_modes(&surface)
            .to_renderer_err("failed to get surface present modes")?;
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
                image_usage: ImageUsage {
                    color_attachment: true,
                    ..ImageUsage::empty()
                },
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

    /// Creates the render target for the scene render. _Note that the value of `access_queue` isn't actually used
    /// in the vulkan image creation create info._
    fn create_render_image(
        access_queue: Arc<Queue>,
        size: [u32; 2],
    ) -> Result<Arc<ImageView<StorageImage>>, RenderManagerError> {
        // format must match what's specified in the compute shader layout
        let render_image_format = Format::R8G8B8A8_UNORM;
        StorageImage::general_purpose_image_view(
            access_queue,
            size,
            render_image_format,
            ImageUsage {
                storage: true,
                sampled: true,
                ..ImageUsage::empty()
            },
        )
        .to_renderer_err("failed to create render image")
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
        self.viewport.dimensions = [resolution[0] as f32, resolution[1] as f32];

        self.render_image = Self::create_render_image(
            self.render_queue.clone(),
            swapchain_images[0].dimensions().width_height(),
        )?;

        // update scene pass
        self.scene_pass
            .update_render_target(resolution, self.render_image.clone())
            .to_renderer_err("failed to update scene pass")?;

        // update blit pass
        self.blit_pass
            .update_render_image(self.render_image.clone())
            .to_renderer_err("failed to update blit pass")?;

        // unset trigger
        self.recreate_swapchain = false;

        Ok(())
    }
}

/// Physical device and queue family indices returned by [`RenderManager::choose_physical_device`]
struct ChoosePhysicalDeviceReturn {
    pub physical_device: Arc<PhysicalDevice>,
    pub render_queue_family: QueueFamilyIndex,
    pub transfer_queue_family: QueueFamilyIndex,
}

/// This mod just makes the module path unique for debug callbacks in the log
mod vulkan_callback {
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
            error!("Vulkan [{}]:\n{}", ty, msg.description);
        } else if msg.severity.warning {
            warn!("Vulkan [{}]:\n{}", ty, msg.description);
        } else if msg.severity.information {
            info!("Vulkan [{}]:\n{}", ty, msg.description);
        } else if msg.severity.verbose {
            debug!("Vulkan [{}]:\n{}", ty, msg.description);
        } else {
            info!("Vulkan [{}] (SEVERITY-UNKONWN):\n{}", ty, msg.description);
        };
    }
}

// ~~~ Errors ~~~

/// Describes the types of errors encountered by the renderer
#[derive(Debug)]
pub enum RenderManagerError {
    /// An unrecoverable/unexpected error has prevented the RenderManager from initializing or rendering.
    /// Contains an string explaining the cause.
    Unrecoverable {
        message: String,
        source: Option<Box<dyn error::Error>>,
    },
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
    // todo VulkanError recoverable case handling...
    // The window surface is no longer accessible and must be recreated.
    // Invalidates the RenderManger and requires re-initialization.
    //
    // Equivalent to vulkano [SurfacePropertiesError::SurfaceLost](`vulkano::device::physical::SurfacePropertiesError::SurfaceLost`)
    //SurfaceLost,
}
impl fmt::Display for RenderManagerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            RenderManagerError::Unrecoverable{message, source} => {
                if let Some(error) = source {
                    write!(f, "{}: {}", message, error)
                } else {
                    write!(f, "{}", message)
                }
            }
            //RenderManagerError::SurfaceLost =>
            //    write!(f, "the Vulkan surface is no longer accessible, thus invalidating this RenderManager instance"),
            RenderManagerError::SurfaceSizeUnsupported{ provided, min_supported, max_supported } =>
                write!(f, "cannot create render target with requested dimensions = {:?}. min size = {:?}, max size = {:?}",
                    provided, min_supported, max_supported),
        }
    }
}
impl error::Error for RenderManagerError {}
impl RenderManagerError {
    /// Passes the error through the `error!` log and returns self
    #[inline]
    pub fn log(self) -> Self {
        error!("{}", self);
        self
    }
}
trait RenderManagerUnrecoverable<T> {
    /// Shorthand for converting a general error to a RenderManagerError::InitFailed.
    /// Commonly used with error propogation `?` in RenderManager::new.
    ///
    /// Similar philosophy to `unwrap` in that these errors are just treated as 'unrecoverable'
    fn to_renderer_err(self, msg: &str) -> Result<T, RenderManagerError>;
}
impl<T, E> RenderManagerUnrecoverable<T> for std::result::Result<T, E>
where
    E: error::Error + 'static,
{
    #[inline]
    #[track_caller]
    fn to_renderer_err(self, msg: &str) -> Result<T, RenderManagerError> {
        match self {
            Ok(x) => Ok(x),
            Err(e) => {
                if config::PANIC_ON_RENDERER_UNRECOVERABLE {
                    error!("{}", e);
                    panic!("{}", e);
                } else {
                    Err(RenderManagerError::Unrecoverable {
                        message: msg.to_string(),
                        source: Some(e.into()),
                    }
                    .log())
                }
            }
        }
    }
}
impl<T> RenderManagerUnrecoverable<T> for std::option::Option<T> {
    #[inline]
    #[track_caller]
    fn to_renderer_err(self, msg: &str) -> Result<T, RenderManagerError> {
        match self {
            Some(x) => Ok(x),
            None => {
                if config::PANIC_ON_RENDERER_UNRECOVERABLE {
                    panic!();
                } else {
                    Err(RenderManagerError::Unrecoverable {
                        message: msg.to_owned(),
                        source: None,
                    }
                    .log())
                }
            }
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
                debug!("cannot create swapchain: {}", err);
                err
            }
            e => Unrecoverable {
                message: "Failed to recreate swapchain".to_string(),
                source: Some(e.into()),
            }
            .log(),
        }
    }
}

/// Describes issues with enabling instance extensions/layers
#[derive(Debug)]
pub enum InstanceSupportError {
    /// Requested instance extension is not supported by this vulkan driver
    ExtensionUnsupported { extension: &'static str },
    /// Requested Vulkan layer is not found (may not be installed)
    LayerNotFound,
    /// Out of memory
    OomError(OomError),
}
impl fmt::Display for InstanceSupportError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            InstanceSupportError::ExtensionUnsupported { extension } => write!(
                f,
                "Requested instance extension {} is not supported by this vulkan driver",
                extension
            ),
            InstanceSupportError::LayerNotFound => write!(
                f,
                "Requested Vulkan layer is not found (may not be installed)"
            ),
            InstanceSupportError::OomError(e) => write!(f, "{}", e),
        }
    }
}
impl error::Error for InstanceSupportError {}
from_err_impl!(InstanceSupportError, OomError);
