use super::{
    blit_pass::BlitPass, gui_renderer::GuiRenderer, overlay_pass::OverlayPass,
    scene_pass::ScenePass,
};
use crate::{
    camera::Camera, config, gui::Gui, primitives::primitive_collection::PrimitiveCollection,
};
use anyhow::{anyhow, bail, ensure, Context};
use log::{debug, error, info, warn};
use std::sync::Arc;
use vulkano::{
    command_buffer::{
        self, allocator::StandardCommandBufferAllocator, RenderPassBeginInfo, SubpassContents,
    },
    descriptor_set::allocator::StandardDescriptorSetAllocator,
    device::{
        self,
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo,
    },
    format::{ClearValue, Format},
    image::{
        view::ImageView, ImageAccess, ImageLayout, ImageUsage, SampleCount, StorageImage,
        SwapchainImage,
    },
    instance::debug::{
        DebugUtilsMessageSeverity, DebugUtilsMessageType, DebugUtilsMessenger,
        DebugUtilsMessengerCreateInfo,
    },
    instance::{Instance, InstanceCreateInfo, InstanceExtensions},
    pipeline::graphics::viewport::Viewport,
    render_pass::{
        AttachmentDescription, AttachmentReference, Framebuffer, FramebufferCreateInfo, LoadOp,
        RenderPass, RenderPassCreateInfo, StoreOp, Subpass, SubpassDependency, SubpassDescription,
    },
    swapchain::{self, Surface, Swapchain, SwapchainCreationError, SwapchainPresentInfo},
    sync::{self, AccessFlags, FlushError, GpuFuture, PipelineStages},
    VulkanLibrary,
};
use winit::window::Window;

/// Indices for render pass attachments and subpasses
mod render_pass_indices {
    pub const ATTACHMENT_SWAPCHAIN: u32 = 0;
    pub const SUBPASS_SWAPCHAIN: u32 = 0;
}

/// Indicates a queue family index
pub type QueueFamilyIndex = u32;

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
    render_pass: Arc<RenderPass>,
    framebuffers: Vec<Arc<Framebuffer>>,

    command_buffer_allocator: StandardCommandBufferAllocator,
    descriptor_allocator: StandardDescriptorSetAllocator,

    scene_pass: ScenePass,
    blit_pass: BlitPass,
    overlay_pass: OverlayPass,
    gui_pass: GuiRenderer,

    future_previous_frame: Option<Box<dyn GpuFuture>>, // todo description
    /// indicates that the swapchain needs to be recreated next frame
    recreate_swapchain: bool,
}

// ~~~ Public functions ~~~

impl RenderManager {
    /// Initializes Vulkan resources. If renderer fails to initialize, returns a string explanation.
    pub fn new(
        window: Arc<Window>,
        primitive_collection: &PrimitiveCollection,
    ) -> anyhow::Result<Self> {
        // load vulkan library
        let vulkan_library = VulkanLibrary::new().context("loading vulkan library")?;
        info!(
            "loaded vulkan library, api version = {}",
            vulkan_library.api_version()
        );

        // required instance extensions for platform surface rendering
        let mut instance_extensions = vulkano_win::required_extensions(&vulkan_library);
        let mut instance_layers: Vec<String> = Vec::new();

        // check for validation layer/debug callback support
        let enable_debug_callback = if config::ENABLE_VULKAN_VALIDATION {
            if add_debug_validation(
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
        debug!("enabling instance extensions: {:?}", instance_extensions);
        debug!("enabling vulkan layers: {:?}", instance_layers);
        let instance = Instance::new(
            vulkan_library.clone(),
            InstanceCreateInfo {
                enabled_extensions: instance_extensions,
                enumerate_portability: true, // enable enumerating devices that use non-conformant vulkan implementations. (ex. MoltenVK)
                enabled_layers: instance_layers,
                ..Default::default()
            },
        )
        .context("creating vulkan instance")?;

        // setup debug callback
        let debug_callback = if enable_debug_callback {
            setup_debug_callback(instance.clone())
        } else {
            None
        };

        // create surface
        let surface = vulkano_win::create_surface_from_winit(window.clone(), instance.clone())
            .context("creating vulkan surface")?;

        // required device extensions
        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };
        debug!("required vulkan device extensions: {:?}", device_extensions);

        // print available physical devices
        debug!("Available Vulkan physical devices:");
        for pd in instance
            .enumerate_physical_devices()
            .context("enumerating physical devices")?
        {
            debug!("\t{}", pd.properties().device_name);
        }
        // choose physical device and queue families
        let ChoosePhysicalDeviceReturn {
            physical_device,
            render_queue_family,
            transfer_queue_family,
        } = choose_physical_device(instance.clone(), &device_extensions, &surface)?;
        info!(
            "Using Vulkan device: {} (type: {:?})",
            physical_device.properties().device_name,
            physical_device.properties().device_type,
        );
        debug!("render queue family index = {}", render_queue_family);
        debug!("transfer queue family index = {}", transfer_queue_family);

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
        let (device, mut queues) = Device::new(
            physical_device.clone(),
            DeviceCreateInfo {
                enabled_extensions: device_extensions,
                enabled_features: device::Features {
                    dynamic_rendering: true,
                    ..device::Features::empty()
                },
                queue_create_infos,
                ..Default::default()
            },
        )
        .context("creating vulkan device and queues")?;
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
            create_swapchain(device.clone(), physical_device.clone(), surface.clone())?;
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
        let swapchain_image_views = swapchain_images
            .iter()
            .map(|image| ImageView::new_default(image.clone()))
            .collect::<Result<Vec<_>, _>>()
            .context("creating swapchain image views")?;

        // scene render target
        let render_image = create_render_image(
            render_queue.clone(),
            swapchain_images[0].dimensions().width_height(),
        )?;

        // create render_pass
        let multisample = SampleCount::Sample1;
        let render_pass = create_render_pass(device.clone(), &swapchain_image_views, multisample)?;

        // describe swapchain subpass
        let subpass = Subpass::from(render_pass.clone(), render_pass_indices::SUBPASS_SWAPCHAIN)
            .ok_or(anyhow!(
                "render pass does not contain subpass at index {}",
                render_pass_indices::SUBPASS_SWAPCHAIN
            ))?;

        // create framebuffers
        let framebuffers = create_framebuffers(render_pass.clone(), &swapchain_image_views)?;

        // command buffer and descriptor allocators
        let command_buffer_allocator = StandardCommandBufferAllocator::new(device.clone());
        let descriptor_allocator = StandardDescriptorSetAllocator::new(device.clone());

        // init compute shader scene pass
        let scene_pass = ScenePass::new(
            device.clone(),
            &descriptor_allocator,
            primitive_collection,
            swapchain_images[0].dimensions().width_height(),
            render_image.clone(),
        )?;

        // init blit pass
        let blit_pass = BlitPass::new(
            device.clone(),
            &descriptor_allocator,
            render_image.clone(),
            subpass.clone(),
        )?;

        // init overlay pass
        let overlay_pass = OverlayPass::new(device.clone(), subpass.clone())?;

        // init gui renderer
        let gui_pass = GuiRenderer::new(device.clone(), transfer_queue.clone(), subpass.clone())?;

        // create futures used for frame synchronization
        let future_previous_frame = Some(sync::now(device.clone()).boxed());

        Ok(RenderManager {
            device,
            render_queue,
            _transfer_queue: transfer_queue,
            _debug_callback: debug_callback,

            surface,
            swapchain,
            swapchain_image_views,

            viewport,
            render_image,
            render_pass,
            framebuffers,

            scene_pass,
            blit_pass,
            overlay_pass,
            gui_pass,

            command_buffer_allocator,
            descriptor_allocator,
            future_previous_frame,
            recreate_swapchain: false,
        })
    }

    /// Submits Vulkan commands for rendering a frame.
    pub fn render_frame(
        &mut self,
        window_resize: bool,
        gui: &mut Gui,
        camera: &Camera,
        primitive_collection: &PrimitiveCollection,
    ) -> anyhow::Result<()> {
        // checks for submission finish and free locks on gpu resources
        if let Some(future_previous_frame) = self.future_previous_frame.as_mut() {
            future_previous_frame.cleanup_finished();
        }

        // update gui textures
        for textures_delta in gui.textures_delta() {
            self.gui_pass.update_textures(
                &self.command_buffer_allocator,
                &self.descriptor_allocator,
                textures_delta,
            )?;
        }

        self.recreate_swapchain = self.recreate_swapchain || window_resize;
        if self.recreate_swapchain {
            // recreate swapchain
            self.recreate_swapchain()?;
        }

        // blocks when no images currently available (all have been submitted already)
        let (swapchain_index, suboptimal, acquire_future) =
            match swapchain::acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => r,
                Err(swapchain::AcquireError::OutOfDate) => {
                    // recreate swapchain and skip frame render
                    return self.recreate_swapchain();
                }
                Err(e) => {
                    return Err(anyhow!(e)).context("aquiring swapchain image");
                }
            };
        // `suboptimal` indicates that the swapchain image will still work but may not be displayed correctly
        // we'll render the frame anyway hehe
        if suboptimal {
            debug!(
                "suboptimal swapchain image {}, rendering anyway...",
                swapchain_index
            );
            self.recreate_swapchain = true;
        }

        // todo shouldn't need to recreate each frame?
        self.scene_pass
            .update_primitives(&self.descriptor_allocator, primitive_collection)?;

        // todo actually set this
        let need_srgb_conv = false;

        // record command buffer
        let mut builder = command_buffer::AutoCommandBufferBuilder::primary(
            &self.command_buffer_allocator,
            self.render_queue.queue_family_index(),
            command_buffer::CommandBufferUsage::OneTimeSubmit,
        )
        .context("beginning primary command buffer")?;

        // compute shader scene render
        self.scene_pass.record_commands(&mut builder, camera)?;

        // begin render pass
        let clear_values: Vec<Option<ClearValue>> = vec![Some([0.0, 0.0, 0.0, 1.0].into())];
        builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values,
                    ..RenderPassBeginInfo::framebuffer(
                        self.framebuffers[swapchain_index as usize].clone(),
                    )
                },
                SubpassContents::Inline,
            )
            .context("recording begin render pass command")?;

        // draw render image to screen
        self.blit_pass
            .record_commands(&mut builder, self.viewport.clone())?;

        // draw editor overlay
        self.overlay_pass.record_commands(
            &mut builder,
            camera,
            primitive_collection,
            self.viewport.clone(),
        )?;

        // render gui
        self.gui_pass.record_commands(
            &mut builder,
            gui,
            need_srgb_conv,
            self.viewport.dimensions,
        )?;

        // end render pass
        builder
            .end_render_pass()
            .context("recording end render pass command")?;
        let command_buffer = builder.build().context("building frame command buffer")?;

        // submit
        let future = self
            .future_previous_frame
            .take()
            .unwrap_or(sync::now(self.device.clone()).boxed()) // should never be None anyway...
            .join(acquire_future)
            .then_execute(self.render_queue.clone(), command_buffer)
            .context("executing vulkan primary command buffer")?
            .then_swapchain_present(
                self.render_queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(
                    self.swapchain.clone(),
                    swapchain_index,
                ),
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
// Private functions
impl RenderManager {
    /// Recreates the swapchain, render image and assiciated descriptor sets, then unsets `recreate_swapchain` trigger.
    fn recreate_swapchain(&mut self) -> anyhow::Result<()> {
        // determine suitable new size
        let new_size: [u32; 2] = self.surface.window().inner_size().into();

        debug!(
            "recreating swapchain and render targets to size: {:?}",
            new_size
        );
        let (new_swapchain, swapchain_images) =
            match self.swapchain.recreate(swapchain::SwapchainCreateInfo {
                image_extent: new_size,
                ..self.swapchain.create_info()
            }) {
                Ok(r) => r,
                // this error tends to happen when the user is manually resizing the window.
                // simply restarting the loop is the easiest way to fix this issue.
                Err(e @ SwapchainCreationError::ImageExtentNotSupported { .. }) => {
                    debug!("failed to recreate swapchain due to {}", e);
                    return Ok(());
                }
                Err(e) => return Err(e).context("recreating swapchain"),
            };

        self.swapchain = new_swapchain;
        self.swapchain_image_views = swapchain_images
            .iter()
            .map(|image| {
                ImageView::new_default(image.clone()).context("recreating swapchaing image view")
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        // set parameters for new resolution
        let resolution = swapchain_images[0].dimensions().width_height();
        self.viewport.dimensions = [resolution[0] as f32, resolution[1] as f32];

        // recreate render image with new dimensions
        self.render_image = create_render_image(
            self.render_queue.clone(),
            swapchain_images[0].dimensions().width_height(),
        )?;

        // recreate render pass and framebuffers for new swapchain images
        self.framebuffers =
            create_framebuffers(self.render_pass.clone(), &self.swapchain_image_views)?;

        // update scene pass
        self.scene_pass.update_render_target(
            &self.descriptor_allocator,
            resolution,
            self.render_image.clone(),
        )?;

        // update blit pass
        self.blit_pass
            .update_render_image(&self.descriptor_allocator, self.render_image.clone())?;

        // unset trigger
        self.recreate_swapchain = false;

        Ok(())
    }
}

/// Checks for VK_EXT_debug_utils support and presence khronos validation layers
/// If both can be enabled, adds them to provided extension and layer lists
fn add_debug_validation(
    vulkan_library: Arc<VulkanLibrary>,
    instance_extensions: &mut InstanceExtensions,
    instance_layers: &mut Vec<String>,
) -> anyhow::Result<()> {
    // check debug utils extension support
    if vulkan_library.supported_extensions().ext_debug_utils {
        info!("VK_EXT_debug_utils was requested and is supported");
    } else {
        warn!("VK_EXT_debug_utils was requested but is unsupported");
        bail!(
            "vulkan extension {} was requested but is unsupported",
            "VK_EXT_debug_utils"
        )
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
        warn!(
            "{} was requested but was not found (may not be installed)",
            validation_layer
        );
        bail!(
            "requested vulkan layer {} not found (may not be installed)",
            validation_layer
        )
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

/// Choose physical device and queue families
fn choose_physical_device(
    instance: Arc<Instance>,
    device_extensions: &DeviceExtensions,
    surface: &Arc<Surface<Arc<Window>>>,
) -> anyhow::Result<ChoosePhysicalDeviceReturn> {
    instance
        .enumerate_physical_devices()
        .context("enumerating physical devices")?
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
                // failed to find suitable main queue
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
        .with_context(|| format!("could not find a suitable vulkan physical device. requirements:\n
            \t- must support minimum vulkan version {}.{}\n
            \t- must contain queue family supporting graphics, compute, transfer and surface operations\n
            \t- must support device extensions: {:?}",
            config::VULKAN_VER_MAJ, config::VULKAN_VER_MIN, device_extensions))
}
/// Physical device and queue family indices returned by [`RenderManager::choose_physical_device`]
struct ChoosePhysicalDeviceReturn {
    pub physical_device: Arc<PhysicalDevice>,
    pub render_queue_family: QueueFamilyIndex,
    pub transfer_queue_family: QueueFamilyIndex,
}

/// Create swapchain and swapchain images
fn create_swapchain(
    device: Arc<Device>,
    physical_device: Arc<PhysicalDevice>,
    surface: Arc<Surface<Arc<Window>>>,
) -> anyhow::Result<(
    Arc<Swapchain<Arc<Window>>>,
    Vec<Arc<SwapchainImage<Arc<Window>>>>,
)> {
    // todo prefer sRGB (linux sRGB)
    let image_format = physical_device
        .surface_formats(&surface, Default::default())
        .context("querying surface formats")?
        .get(0)
        .expect("vulkan driver should support at least 1 surface format... right?")
        .0;
    debug!("swapchain image format = {:?}", image_format);

    let surface_capabilities = physical_device
        .surface_capabilities(&surface, Default::default())
        .context("querying surface capabilities")?;
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
        .context("querying surface present modes")?;
    let present_mode = present_modes
        .find(|&pm| pm == swapchain::PresentMode::Mailbox)
        .unwrap_or(swapchain::PresentMode::Fifo);
    debug!("swapchain present mode = {:?}", present_mode);

    swapchain::Swapchain::new(
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
    )
    .context("creating swapchain")
}

/// Creates the render target for the scene render. _Note that the value of `access_queue` isn't actually used
/// in the vulkan image creation create info._
fn create_render_image(
    access_queue: Arc<Queue>,
    size: [u32; 2],
) -> anyhow::Result<Arc<ImageView<StorageImage>>> {
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
    .context("creating render image")
}

/// Create render pass
fn create_render_pass(
    device: Arc<Device>,
    swapchain_image_views: &Vec<Arc<ImageView<SwapchainImage<Arc<Window>>>>>,
    swapchain_sample_count: SampleCount,
) -> anyhow::Result<Arc<RenderPass>> {
    ensure!(
        swapchain_image_views.len() >= 1,
        "no swapchain images provided to create render pass"
    );
    let swapchain_image = &swapchain_image_views[0].image();

    let attachments: Vec<AttachmentDescription> = vec![
        // swapchain image
        AttachmentDescription {
            format: Some(swapchain_image.format()),
            samples: swapchain_sample_count,
            load_op: LoadOp::Clear,
            store_op: StoreOp::Store,
            initial_layout: ImageLayout::PresentSrc,
            final_layout: ImageLayout::PresentSrc,
            ..Default::default()
        },
    ];

    let subpasses = vec![
        // blit + gui passes
        SubpassDescription {
            color_attachments: vec![Some(AttachmentReference {
                attachment: render_pass_indices::ATTACHMENT_SWAPCHAIN,
                layout: ImageLayout::ColorAttachmentOptimal,
                ..Default::default()
            })],
            ..Default::default()
        },
    ];

    let dependencies = vec![
        // wait for swapchain to finish reading from image before rendering (vulkano probably adds some conservative sync anyway tho...)
        SubpassDependency {
            source_subpass: None, // = VK_SUBPASS_EXTERNAL
            destination_subpass: Some(render_pass_indices::SUBPASS_SWAPCHAIN),
            source_stages: PipelineStages {
                color_attachment_output: true,
                ..Default::default()
            },
            source_access: AccessFlags::empty(),
            destination_stages: PipelineStages {
                color_attachment_output: true,
                ..Default::default()
            },
            destination_access: AccessFlags {
                color_attachment_write: true,
                ..Default::default()
            },
            ..Default::default()
        },
    ];

    RenderPass::new(
        device,
        RenderPassCreateInfo {
            attachments,
            subpasses,
            dependencies,
            ..Default::default()
        },
    )
    .context("creating vulkan render pass")
}

/// Create swapchain image framebuffers
fn create_framebuffers(
    render_pass: Arc<RenderPass>,
    swapchain_image_views: &Vec<Arc<ImageView<SwapchainImage<Arc<Window>>>>>,
) -> anyhow::Result<Vec<Arc<Framebuffer>>> {
    swapchain_image_views
        .iter()
        .map(|image_view| {
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![image_view.clone()],
                    ..Default::default()
                },
            )
            .context("creating vulkan framebuffer")
        })
        .collect::<anyhow::Result<Vec<_>>>()
}

/// This mod just makes the module path unique for debug callbacks in the log
mod vulkan_callback {
    use log::{debug, error, warn};
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
            debug!("Vulkan [{}]:\n{}", ty, msg.description);
        } else if msg.severity.verbose {
            debug!("Vulkan [{}]:\n{}", ty, msg.description);
        } else {
            debug!("Vulkan [{}] (SEVERITY-UNKONWN):\n{}", ty, msg.description);
        };
    }
}
