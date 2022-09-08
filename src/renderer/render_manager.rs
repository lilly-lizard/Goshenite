// todo handle unwraps/panics/expects (e.g. clean exit) and error propagation

use crate::{camera::Camera, config, shaders::shader_interfaces};
use glam::Mat4;
#[allow(unused_imports)]
use log::{debug, error, info, warn};
use std::{error, fmt, fs, sync::Arc};
use vulkano::{
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, RenderingAttachmentInfo, RenderingInfo,
    },
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType, SurfacePropertiesError},
        Device, DeviceCreateInfo, DeviceExtensions, Features, Queue, QueueCreateInfo,
    },
    format::Format,
    image::{view::ImageView, ImageAccess, ImageUsage, StorageImage, SwapchainImage},
    instance::{
        debug::{
            DebugUtilsMessageSeverity, DebugUtilsMessageType, DebugUtilsMessenger,
            DebugUtilsMessengerCreateInfo,
        },
        layers_list, Instance, InstanceCreateInfo, InstanceExtensions,
    },
    pipeline::{
        graphics::{
            render_pass::PipelineRenderingCreateInfo,
            viewport::{Viewport, ViewportState},
        },
        ComputePipeline, GraphicsPipeline, Pipeline, PipelineBindPoint,
    },
    render_pass::{LoadOp, StoreOp},
    sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo},
    shader::ShaderModule,
    swapchain::{
        acquire_next_image, AcquireError, CompositeAlpha, Surface, Swapchain, SwapchainCreateInfo,
        SwapchainCreationError,
    },
    sync::{self, FlushError, GpuFuture},
    Version,
};
use vulkano_win::create_surface_from_winit;
use winit::window::Window;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderManagerError {
    /// Vulkan renderer is unable to initialize, a string containing the reason is included
    InitFailed(String),

    /// An unrecoverable or unexpected error occured while rendering a frame
    RenderFrameFailed(String),

    /// The window surface is no longer accessible and must be recreated.
    /// Invalidates the RenderManger and requires re-initialization.
    /// Equivalent to vulkano::device::physical::SurfacePropertiesError::SurfaceLost
    SurfaceLost,

    /// Requested dimensions are not within supported range when attempting to create a render target (swapchain)
    /// Equivalent to vulkano::swapchain::SwapchainCreationError::ImageExtentNotSupported
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
            RenderManagerError::InitFailed(msg) => write!(fmt, "failed to initialize RenderManager: {}", msg),
            RenderManagerError::RenderFrameFailed(msg) => write!(fmt, "failed to render frame {}", msg),
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
pub trait RenderManagerInitErr<T> {
    fn init_err(self, msg: &str) -> Result<T, RenderManagerError>;
}
impl<T, E> RenderManagerInitErr<T> for std::result::Result<T, E>
where
    E: fmt::Debug,
{
    #[inline]
    #[track_caller]
    fn init_err(self, msg: &str) -> Result<T, RenderManagerError> {
        match self {
            Ok(x) => Ok(x),
            Err(e) => Err(RenderManagerError::InitFailed(format!("{}: {:?}", msg, e)).log()),
        }
    }
}

// Members
pub struct RenderManager {
    _debug_callback: Option<DebugUtilsMessenger>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    surface: Arc<Surface<Arc<Window>>>,
    swapchain: Arc<Swapchain<Arc<Window>>>,
    swapchain_image_views: Vec<Arc<ImageView<SwapchainImage<Arc<Window>>>>>,
    viewport: Viewport,
    render_image: Arc<ImageView<StorageImage>>,
    render_image_format: Format,
    sampler: Arc<Sampler>,
    pipeline_compute: Arc<ComputePipeline>,
    pipeline_post: Arc<GraphicsPipeline>,
    desc_set_render: Arc<PersistentDescriptorSet>,
    desc_set_post: Arc<PersistentDescriptorSet>,
    work_group_size: [u32; 2],
    work_group_count: [u32; 3],
    future_previous_frame: Option<Box<dyn GpuFuture>>, // todo description
    recreate_swapchain: bool, // indicates that the swapchain needs to be recreated next frame
}
// Public functions
impl RenderManager {
    /// Initializes Vulkan resources. If renderer fails to initialize, returns a string explanation.
    pub fn new(window: Arc<Window>) -> Result<Self, RenderManagerError> {
        use RenderManagerError::{InitFailed, SurfaceSizeUnsupported};

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
            enabled_layers: instance_layers,
            ..Default::default()
        })
        .init_err("Failed to create vulkan instance")?;

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

        let surface = match create_surface_from_winit(window, instance.clone()) {
            Ok(s) => s,
            Err(e) => {
                return Err(InitFailed(format!("failed to create vulkan surface: {:?}", e)).log());
            }
        };

        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::none()
        };

        // print available devices
        debug!("Available Vulkan physical devices:");
        for pd in PhysicalDevice::enumerate(&instance) {
            debug!("\t{}", pd.properties().device_name);
        }
        // choose physical device and queue family
        let (physical_device, queue_family) = match PhysicalDevice::enumerate(&instance)
            // filter for vulkan version support
            .filter(|&p| {
                p.api_version()
                    >= Version::major_minor(config::VULKAN_VER_MAJ, config::VULKAN_VER_MIN)
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
            }) {
            Some(x) => x,
            None => {
                return Err(InitFailed("no suitable physical device available".to_owned()).log())
            }
        };
        info!(
            "Using Vulkan device: {} (type: {:?})",
            physical_device.properties().device_name,
            physical_device.properties().device_type,
        );

        let (device, mut queues) = match Device::new(
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
        ) {
            Ok(x) => x,
            Err(e) => {
                return Err(InitFailed(format!("failed to create vulkan device: {:?}", e)).log())
            }
        };

        let queue = queues.next().expect(
            "vulkano::device::Device::new has an assert to ensure at least 1 queue gets created",
        );

        // todo prefer sRGB? (linux sRGB)
        let (swapchain, swapchain_images) = {
            let surface_capabilities = match physical_device
                .surface_capabilities(&surface, Default::default())
            {
                Ok(x) => x,
                Err(SurfacePropertiesError::SurfaceLost) => {
                    return Err(RenderManagerError::SurfaceLost.log())
                }
                Err(e) => {
                    return Err(
                        InitFailed(format!("failed to get surface capabilities: {:?}", e,)).log(),
                    )
                }
            };
            let swapchain_image_format = match physical_device
                .surface_formats(&surface, Default::default())
            {
                Ok(x) => x,
                Err(SurfacePropertiesError::SurfaceLost) => {
                    return Err(RenderManagerError::SurfaceLost)
                }
                Err(e) => {
                    return Err(InitFailed(format!("failed to get surface format: {:?}", e)).log())
                }
            }
            .get(0)
            .expect("vulkan driver should support at least 1 surface format... right?")
            .0;

            let composite_alpha = surface_capabilities
                .supported_composite_alpha
                .iter()
                .max_by_key(|c| match c {
                    CompositeAlpha::PostMultiplied => 4,
                    CompositeAlpha::Inherit => 3,
                    CompositeAlpha::Opaque => 2,
                    CompositeAlpha::PreMultiplied => 1, // because cbf implimenting this logic
                    _ => 0,
                })
                .expect("surface should support at least 1 composite mode... right?");

            match Swapchain::new(
                device.clone(),
                surface.clone(),
                SwapchainCreateInfo {
                    min_image_count: surface_capabilities.min_image_count,
                    image_format: Some(swapchain_image_format),
                    image_extent: surface.window().inner_size().into(),
                    image_usage: ImageUsage::color_attachment(),
                    composite_alpha,
                    ..Default::default()
                },
            ) {
                Ok(x) => x,
                Err(SwapchainCreationError::ImageExtentNotSupported {
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
                    return Err(InitFailed(format!("failed to create swapchain: {:?}", e)).log())
                }
            }
        };

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
        let swapchain_image_views = match swapchain_image_views {
            Ok(x) => x,
            Err(e) => {
                return Err(
                    InitFailed(format!("failed to create swapchain image view(s) {:?}", e)).log(),
                )
            }
        };

        // compute shader render target
        let render_image_format = Format::R8G8B8A8_UNORM;
        let render_image = match StorageImage::general_purpose_image_view(
            queue.clone(),
            swapchain_images[0].dimensions().width_height(),
            render_image_format,
            ImageUsage {
                storage: true,
                sampled: true,
                ..ImageUsage::none()
            },
        ) {
            Ok(x) => x,
            Err(e) => {
                return Err(InitFailed(format!("failed to create render image: {:?}", e)).log())
            }
        };

        let sampler = match Sampler::new(
            device.clone(),
            SamplerCreateInfo {
                mag_filter: Filter::Linear,
                min_filter: Filter::Linear,
                address_mode: [SamplerAddressMode::Repeat; 3],
                ..Default::default()
            },
        ) {
            Ok(x) => x,
            Err(e) => return Err(InitFailed(format!("failed to create sampler: {:?}", e)).log()),
        };

        let shader_render = match fs::read("assets/shader_binaries/render.comp.spv") {
            Ok(x) => x,
            Err(e) => return Err(InitFailed(format!("render.comp.spv read failed: {:?}", e)).log()),
        };
        let shader_post_vert = match fs::read("assets/shader_binaries/post.vert.spv") {
            Ok(x) => x,
            Err(e) => return Err(InitFailed(format!("post.vert.spv read failed: {:?}", e)).log()),
        };
        let shader_post_frag = match fs::read("assets/shader_binaries/post.frag.spv") {
            Ok(x) => x,
            Err(e) => return Err(InitFailed(format!("post.frag.spv read failed: {:?}", e)).log()),
        };

        // todo conv to &[u32] and use from_words
        let shader_render = match unsafe {
            ShaderModule::from_bytes(device.clone(), shader_render.as_slice())
        } {
            Ok(x) => x,
            Err(e) => {
                return Err(InitFailed(format!("render.comp shader compile failed: {:?}", e)).log())
            }
        };
        let shader_post_vert = match unsafe {
            ShaderModule::from_bytes(device.clone(), shader_post_vert.as_slice())
        } {
            Ok(x) => x,
            Err(e) => {
                return Err(InitFailed(format!("post.vert shader compile failed: {:?}", e)).log())
            }
        };
        let shader_post_frag = match unsafe {
            ShaderModule::from_bytes(device.clone(), shader_post_frag.as_slice())
        } {
            Ok(x) => x,
            Err(e) => {
                return Err(InitFailed(format!("post.frag shader compile failed: {:?}", e)).log())
            }
        };

        let pipeline_compute = ComputePipeline::new(
            device.clone(),
            match shader_render.entry_point("main") {
                Some(x) => x,
                None => return Err(InitFailed("no main in render.comp".to_owned()).log()),
            },
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

        let pipeline_post = GraphicsPipeline::start()
            .render_pass(PipelineRenderingCreateInfo {
                color_attachment_formats: vec![Some(swapchain.image_format())],
                ..Default::default()
            })
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .vertex_shader(
                match shader_post_vert.entry_point("main") {
                    Some(x) => x,
                    None => return Err(InitFailed("no main in post.vert".to_owned()).log()),
                },
                (),
            )
            .fragment_shader(
                match shader_post_frag.entry_point("main") {
                    Some(x) => x,
                    None => return Err(InitFailed("no main in post.frag".to_owned()).log()),
                },
                (),
            )
            .build(device.clone())
            .unwrap();

        let desc_set_render = PersistentDescriptorSet::new(
            pipeline_compute
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

        let desc_set_post = PersistentDescriptorSet::new(
            pipeline_post
                .layout()
                .set_layouts()
                .get(shader_interfaces::descriptor::SET_POST_FRAG)
                .unwrap()
                .to_owned(),
            [WriteDescriptorSet::image_view_sampler(
                shader_interfaces::descriptor::BINDING_SAMPLER,
                render_image.clone(),
                sampler.clone(),
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
            sampler,
            pipeline_compute,
            pipeline_post,
            desc_set_render,
            desc_set_post,
            work_group_size,
            work_group_count,
            future_previous_frame,
            recreate_swapchain,
        })
    }

    /// Submits Vulkan commands for rendering a frame.
    /// Blocks until previous frame has finished rendering todo efficient
    pub fn render_frame(&mut self, window_resize: bool, camera: Camera) -> Result<(), String> {
        // checks for submission finish and free locks on gpu resources
        self.future_previous_frame
            .as_mut()
            .unwrap()
            .cleanup_finished();

        self.recreate_swapchain = self.recreate_swapchain || window_resize;
        if self.recreate_swapchain {
            self.recreate_swapchain(); // todo error handle/propogation
        }

        // blocks when no images currently available (all have been submitted already)
        let (image_num, suboptimal, acquire_future) =
            match acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    self.recreate_swapchain = true;
                    debug!("out of date swapchain, recreating...");
                    self.recreate_swapchain(); // todo error handle/propogation
                    return Ok(());
                }
                Err(e) => {
                    todo!("handle recovery cases, e.g. timeout, surface recreate...");
                    return Err(format!("Failed to acquire next image: {:?}", e));
                }
            };
        if suboptimal {
            self.recreate_swapchain = true;
        }

        let render_push_constants = shader_interfaces::CameraPc::new(
            Mat4::inverse(&(camera.proj_matrix() * camera.view_matrix())),
            camera.position(),
        );

        // record command buffer
        let mut builder = AutoCommandBufferBuilder::primary(
            self.device.clone(),
            self.queue.family(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();
        builder
            .bind_pipeline_compute(self.pipeline_compute.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Compute,
                self.pipeline_compute.layout().clone(),
                0,
                self.desc_set_render.clone(),
            )
            .push_constants(
                self.pipeline_compute.layout().clone(),
                0,
                render_push_constants,
            )
            .dispatch(self.work_group_count)
            .unwrap()
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
            .bind_pipeline_graphics(self.pipeline_post.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline_post.layout().clone(),
                0,
                self.desc_set_post.clone(),
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
        use RenderManagerError::{RenderFrameFailed, SurfaceSizeUnsupported};

        let (new_swapchain, swapchain_images) = match self.swapchain.recreate(SwapchainCreateInfo {
            image_extent: self.surface.window().inner_size().into(),
            ..self.swapchain.create_info()
        }) {
            Ok(r) => r,
            // this error tends to happen when the user is manually resizing the window.
            // simply restarting the loop is the easiest way to fix this issue.
            Err(SwapchainCreationError::ImageExtentNotSupported {
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
                    RenderFrameFailed(format!("Failed to recreate swapchain: {:?}", e)).log(),
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

        self.desc_set_render = PersistentDescriptorSet::new(
            self.pipeline_compute
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

        self.desc_set_post = PersistentDescriptorSet::new(
            self.pipeline_post
                .layout()
                .set_layouts()
                .get(shader_interfaces::descriptor::SET_POST_FRAG)
                .unwrap()
                .to_owned(),
            [WriteDescriptorSet::image_view_sampler(
                shader_interfaces::descriptor::BINDING_SAMPLER,
                self.render_image.clone(),
                self.sampler.clone(),
            )],
        )
        .unwrap();

        // unset trigger
        self.recreate_swapchain = false;

        Ok(())
    }
}

// Helper functions

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
        group_count_x = group_count_x + 1;
    }
    let mut group_count_y = resolution[1] / work_group_size[1];
    if (resolution[1] % work_group_size[1]) != 0 {
        group_count_y = group_count_y + 1;
    }
    [group_count_x, group_count_y, 1]
}
