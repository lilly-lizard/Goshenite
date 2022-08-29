// todo handle unwraps/panics/expects (e.g. clean exit) and error propagation
// todo moooore debug logs

use crate::config;
#[allow(unused_imports)]
use log::{debug, error, info, warn}; // todo "Renderer" message prefix
use std::sync::Arc;
use vulkano::{
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, RenderingAttachmentInfo, RenderingInfo,
    },
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceCreateInfo, DeviceExtensions, Features, Queue, QueueCreateInfo,
    },
    image::{view::ImageView, ImageAccess, ImageUsage, SwapchainImage},
    instance::{
        debug::{
            DebugUtilsMessageSeverity, DebugUtilsMessageType, DebugUtilsMessenger,
            DebugUtilsMessengerCreateInfo, Message,
        },
        layers_list, Instance, InstanceCreateInfo, InstanceExtensions,
    },
    pipeline::{
        graphics::{
            render_pass::PipelineRenderingCreateInfo,
            viewport::{Viewport, ViewportState},
        },
        GraphicsPipeline,
    },
    render_pass::{LoadOp, StoreOp},
    shader::ShaderModule,
    swapchain::{
        acquire_next_image, AcquireError, Surface, Swapchain, SwapchainCreateInfo,
        SwapchainCreationError,
    },
    sync::{self, FlushError, GpuFuture},
    Version,
};
use vulkano_win::create_surface_from_winit;
use winit::window::Window;

// RenderManager members

pub struct RenderManager {
    debug_callback: Option<DebugUtilsMessenger>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    surface: Arc<Surface<Arc<Window>>>,
    swapchain: Arc<Swapchain<Arc<Window>>>,
    swapchain_image_views: Vec<Arc<ImageView<SwapchainImage<Arc<Window>>>>>,
    viewport: Viewport,
    pipeline_graphics: Arc<GraphicsPipeline>,
    future_previous_frame: Option<Box<dyn GpuFuture>>, // todo description
    recreate_swapchain: bool, // indicates that the swapchain needs to be recreated next frame
}

// RenderManager public functions

impl RenderManager {
    pub fn new(window: Arc<Window>) -> Self {
        let mut instance_extensions = vulkano_win::required_extensions();
        let mut instance_layers: Vec<String> = Vec::new();

        // check for validation layer/debug callback support
        let enable_debug_callback =
            if let Ok(_) = add_debug_validation(&mut instance_extensions, &mut instance_layers) {
                info!("enabling Vulkan validation layers and debug callback");
                true
            } else {
                warn!("validation layer debug callback requested but cannot be enabled");
                false
            };

        // create instance
        let instance = Instance::new(InstanceCreateInfo {
            enabled_extensions: instance_extensions,
            enumerate_portability: true, // enable enumerating devices that use non-conformant vulkan implementations. (ex. MoltenVK)
            ..Default::default()
        })
        .unwrap();

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
                            process_debug_callback(msg)
                        }))
                    },
                )
                .ok()
            }
        } else {
            None
        };

        let surface = create_surface_from_winit(window, instance.clone()).unwrap();

        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::none()
        };

        let (physical_device, queue_family) = PhysicalDevice::enumerate(&instance)
            .filter(|&p| {
                p.api_version()
                    >= Version::major_minor(config::VULKAN_VER_MAJ, config::VULKAN_VER_MIN)
            })
            .filter(|&p| p.supported_extensions().is_superset_of(&device_extensions))
            .filter_map(|p| {
                p.queue_families()
                    .find(|&q| {
                        q.supports_compute()
                            && q.supports_graphics()
                            && q.supports_surface(&surface).unwrap_or(false)
                    })
                    .map(|q| (p, q))
            })
            // device type preference
            .min_by_key(|(p, _)| match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                PhysicalDeviceType::Other => 4,
            })
            .expect("No suitable physical device found");
        info!(
            "Using vulkan device: {} (type: {:?})",
            physical_device.properties().device_name,
            physical_device.properties().device_type,
        );

        let (device, mut queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                enabled_extensions: device_extensions,
                enabled_features: Features {
                    dynamic_rendering: true,
                    ..Features::none()
                },
                queue_create_infos: vec![QueueCreateInfo::family(queue_family)],
                ..Default::default()
            },
        )
        .unwrap();

        let queue = queues.next().unwrap();

        let (swapchain, swapchain_images) = {
            let surface_capabilities = physical_device
                .surface_capabilities(&surface, Default::default())
                .unwrap();
            let image_format = Some(
                physical_device
                    .surface_formats(&surface, Default::default())
                    .unwrap()[0]
                    .0,
            );
            Swapchain::new(
                device.clone(),
                surface.clone(),
                SwapchainCreateInfo {
                    min_image_count: surface_capabilities.min_image_count,
                    image_format,
                    image_extent: surface.window().inner_size().into(),
                    image_usage: ImageUsage::color_attachment(),
                    composite_alpha: surface_capabilities
                        .supported_composite_alpha
                        .iter()
                        .next()
                        .unwrap(),
                    ..Default::default()
                },
            )
            .unwrap()
        };

        // todo safe spirv loading
        let vs = unsafe {
            ShaderModule::from_bytes(
                device.clone(),
                // load spv at runtime
                std::fs::read("assets/shader_binaries/post.vert.spv")
                    .unwrap()
                    .as_slice(),
            )
        }
        .unwrap();
        let fs = unsafe {
            ShaderModule::from_bytes(
                device.clone(),
                // load spv at runtime
                std::fs::read("assets/shader_binaries/post.frag.spv")
                    .unwrap()
                    .as_slice(),
            )
        }
        .unwrap();

        let pipeline_graphics = GraphicsPipeline::start()
            .render_pass(PipelineRenderingCreateInfo {
                color_attachment_formats: vec![Some(swapchain.image_format())],
                ..Default::default()
            })
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .vertex_shader(vs.entry_point("main").unwrap(), ())
            .fragment_shader(fs.entry_point("main").unwrap(), ())
            .build(device.clone())
            .unwrap();

        // dynamic viewport
        let mut viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [0.0, 0.0],
            depth_range: 0.0..1.0,
        };

        // swapchain image views
        let swapchain_image_views = window_size_dependent_setup(&swapchain_images, &mut viewport);

        let future_previous_frame = Some(sync::now(device.clone()).boxed());
        let recreate_swapchain = false;

        RenderManager {
            debug_callback,
            device,
            queue,
            surface,
            swapchain,
            swapchain_image_views,
            viewport,
            pipeline_graphics,
            future_previous_frame,
            recreate_swapchain,
        }
    }

    pub fn render_frame(&mut self, window_resize: bool) {
        // checks for submission finish and free locks on gpu resources
        self.future_previous_frame
            .as_mut()
            .unwrap()
            .cleanup_finished();

        self.recreate_swapchain = self.recreate_swapchain || window_resize;
        // recreate swapchain
        if self.recreate_swapchain {
            let (new_swapchain, new_images) = match self.swapchain.recreate(SwapchainCreateInfo {
                image_extent: self.surface.window().inner_size().into(),
                ..self.swapchain.create_info()
            }) {
                Ok(r) => r,
                // this error tends to happen when the user is manually resizing the window.
                // simply restarting the loop is the easiest way to fix this issue.
                Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
                Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
            };

            self.swapchain = new_swapchain;
            self.swapchain_image_views =
                window_size_dependent_setup(&new_images, &mut self.viewport);
        }
        self.recreate_swapchain = false;

        // blocks when no images currently available (all have been submitted already)
        let (image_num, suboptimal, acquire_future) =
            // todo timeout for no images case...
            match acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    self.recreate_swapchain = true;
                    return;
                }
                Err(e) => panic!("Failed to acquire next image: {:?}", e),
            };
        if suboptimal {
            self.recreate_swapchain = true;
        }

        let mut builder = AutoCommandBufferBuilder::primary(
            self.device.clone(),
            self.queue.family(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        let bruh: Arc<dyn vulkano::image::view::ImageViewAbstract> =
            self.swapchain_image_views[image_num].clone();

        builder
            .begin_rendering(RenderingInfo {
                color_attachments: vec![Some(RenderingAttachmentInfo {
                    load_op: LoadOp::Clear,
                    store_op: StoreOp::Store,
                    clear_value: Some([0.0, 1.0, 0.0, 1.0].into()),
                    ..RenderingAttachmentInfo::image_view(
                        self.swapchain_image_views[image_num].clone(),
                    )
                })],
                ..Default::default()
            })
            .unwrap()
            .set_viewport(0, [self.viewport.clone()])
            .bind_pipeline_graphics(self.pipeline_graphics.clone())
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
    }

    pub fn wait_device(&self) {
        todo!();
    }
}

// Helper functions

/// Prints/logs a Vulkan validation layer message
fn process_debug_callback(msg: &Message) {
    let ty = if msg.ty.general {
        "general"
    } else if msg.ty.validation {
        "validation"
    } else if msg.ty.performance {
        "performance"
    } else {
        "type unknown"
    };

    if msg.severity.error {
        error!("Vulkan {} [{}]: {}", "ERROR", ty, msg.description);
    } else if msg.severity.warning {
        warn!("Vulkan {} [{}]: {}", "WARNING", ty, msg.description);
    } else if msg.severity.information {
        info!("Vulkan {} [{}]: {}", "INFO", ty, msg.description);
    } else if msg.severity.verbose {
        debug!("Vulkan {} [{}]: {}", "VERBOSE", ty, msg.description);
    } else {
        debug!(
            "Vulkan {} [{}]: {}",
            "[unkown severity]", ty, msg.description
        );
    };
}

/// Describes issues with enabling instance extensions/layers
enum InstanceSupportError {
    /// Requested instance extension is not supported by this vulkan driver
    ExtensionUnsupported,
    /// Requested Vulkan layer is not found (may not be installed)
    LayerNotFound,
}
/// Checks for VK_EXT_debug_utils support and presence khronos validation layers
/// If both can be enabled, adds them to provided extension and layer lists
fn add_debug_validation(
    instance_extensions: &mut InstanceExtensions,
    instance_layers: &mut Vec<String>,
) -> Result<(), InstanceSupportError> {
    // check debug utils extension support
    if match InstanceExtensions::supported_by_core() {
        Ok(supported) => supported.ext_debug_utils,
        Err(_) => false,
    } {
        info!("VK_EXT_debug_utils was requested and is supported");
    } else {
        warn!("VK_EXT_debug_utils was requested but is unsupported");
        return Err(InstanceSupportError::ExtensionUnsupported);
    }

    // check validation layers are present
    #[cfg(not(target_os = "macos"))]
    let validation_layer = "VK_LAYER_LUNARG_standard_validation";
    #[cfg(target_os = "macos")]
    let validation_layer = "VK_LAYER_KHRONOS_validation";
    if {
        let available_layers = layers_list().expect("failed to open vulkan library");
        let mut layer_found = false;
        for l in available_layers {
            if validation_layer == l.name() {
                layer_found = true;
                break;
            }
        }
        layer_found
    } {
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

/// This method is called once during initialization, then again whenever the window is resized
/// todo refactor?
fn window_size_dependent_setup<W: vulkano_win::SafeBorrow<Window>>(
    images: &Vec<Arc<SwapchainImage<W>>>,
    viewport: &mut Viewport,
) -> Vec<Arc<ImageView<SwapchainImage<W>>>>
where
    SwapchainImage<W>: ImageAccess,
{
    let dimensions = images[0].dimensions().width_height();
    viewport.dimensions = [dimensions[0] as f32, dimensions[1] as f32];

    images
        .iter()
        .map(|image| ImageView::new_default(image.clone()).unwrap())
        .collect::<Vec<_>>()
}
