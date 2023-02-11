use super::{
    config_renderer::{
        ENABLE_VULKAN_VALIDATION, FORMAT_DEPTH_BUFFER, FORMAT_G_BUFFER_NORMAL, FRAMES_IN_FLIGHT,
    },
    geometry_pass::GeometryPass,
    gui_renderer::GuiRenderer,
    lighting_pass::LightingPass,
    shader_interfaces::{
        primitive_op_buffer::primitive_codes, uniform_buffers::CameraUniformBuffer,
    },
};
use crate::{
    config::ENGINE_NAME,
    engine::object::{object_collection::ObjectCollection, objects_delta::ObjectsDelta},
    renderer::config_renderer::{FORMAT_G_BUFFER_PRIMITIVE_ID, VULKAN_VER_MAJ, VULKAN_VER_MIN},
    user_interface::{camera::Camera, gui::Gui},
};
use anyhow::{anyhow, Context};
use ash::{
    vk::{self, DebugUtilsMessageSeverityFlagsEXT, PhysicalDeviceVulkan12Features, QueueFlags},
};
use bort::{
    debug_callback::DebugCallback,
    instance::{ApiVersion, Instance},
    physical_device::PhysicalDevice,
    surface::Surface,
};
use egui::TexturesDelta;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::{borrow::Cow, ffi::CStr, sync::Arc};
use winit::window::Window;

// number of primary and secondary command buffers to initially allocate
const PRE_ALLOCATE_PRIMARY_COMMAND_BUFFERS: usize = 64;
const PRE_ALLOCATE_SECONDARY_COMMAND_BUFFERS: usize = 0;

// todo move these somewhere else

unsafe extern "system" fn log_vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;

    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    match message_severity {
        DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            error!("Vulkan [{:?}]:\n{}", message_type, message);
        }
        DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            warn!("Vulkan [{:?}]:\n{}", message_type, message);
        }
        DebugUtilsMessageSeverityFlagsEXT::INFO => {
            info!("Vulkan [{:?}]:\n{}", message_type, message);
        }
        DebugUtilsMessageSeverityFlagsEXT::VERBOSE => {
            trace!("Vulkan [{:?}]:\n{}", message_type, message);
        }
        _ => trace!(
            "Vulkan [{:?}] (UNKONWN SEVERITY):\n{}",
            message_type,
            message
        ),
    }

    vk::FALSE
}

fn required_device_extensions() -> [String] {
    [
        "VK_KHR_swapchain".to_string(),
        "VK_EXT_descriptor_indexing".to_string(),
    ]
}

/// Make sure to update `required_features` too!
fn supports_required_features(supported_features: PhysicalDeviceVulkan12Features) -> bool {
    supported_features.descriptor_indexing
        && supported_features.runtime_descriptor_array
        && supported_features.descriptor_binding_variable_descriptor_count
        && supported_features.shader_storage_buffer_array_non_uniform_indexing
        && supported_features.descriptor_binding_partially_bound
}
/// Make sure to update `supports_required_features` too!
fn required_features() -> PhysicalDeviceVulkan12Features {
    PhysicalDeviceVulkan12Features {
        descriptor_indexing: true,
        runtime_descriptor_array: true,
        descriptor_binding_variable_descriptor_count: true,
        shader_storage_buffer_array_non_uniform_indexing: true,
        descriptor_binding_partially_bound: true,
        ..PhysicalDeviceVulkan12Features::default()
    }
}

struct ChoosePhysicalDeviceReturn {
    pub physical_device: PhysicalDevice,
    pub render_queue_family_index: usize,
    pub transfer_queue_family_index: usize,
}
fn choose_physical_device_and_queue_families(
    instance: &Instance,
    surface: &Surface,
) -> anyhow::Result<ChoosePhysicalDeviceReturn> {
    let p_device_handles = unsafe { instance.inner().enumerate_physical_devices() }
        .context("enumerating physical devices")?;
    let p_devices: Vec<PhysicalDevice> = p_device_handles
        .iter()
        .map(|handle| PhysicalDevice::new(instance, handle))
        .collect();

    let required_extensions = required_device_extensions();
    let required_features = required_features();
    debug!(
        "choosing physical device... required features: {:?}",
        required_features
    );

    let chosen_device = p_devices
        .iter()
        // filter for supported api version
        .filter(|p| p.supports_api_ver(instance.api_version()))
        // filter for required device extensionssupports_extension
        .filter(|p| p.supports_extensions(required_extensions))
        // filter for queue support
        .filter_map(|p| {
            // get queue family index for main queue
            let render_family = p
                .queue_family_properties()
                .iter()
                // because we want the queue family index
                .enumerate()
                .position(|(i, q)| {
                    // must support our surface and essential operations
                    q.queue_flags.contains(QueueFlags::GRAPHICS)
                        && q.queue_flags.contains(QueueFlags::TRANSFER)
                        && surface
                            .get_physical_device_surface_support(p, i)
                            .unwrap_or(false)
                });
            let render_family = match render_family {
                Some(x) => x,
                None => {
                    debug!("no suitable queue family index found for physical device {}", p.properties().device_name);
                    return None;
                },
            };

            // check requried device features support
            let supported_features = p.features();
            if supports_required_features(supported_features) {
                debug!(
                    "physical device {} doesn't support required features. supported features: {:?}",
                    p.properties().device_name,
                    supported_features
                );
                return None;
            }

            // attempt to find a different queue family that we can use for asynchronous transfer operations
            // e.g. uploading image/buffer data at same time as rendering
            let transfer_family = p
                .queue_family_properties()
                .iter()
                .enumerate()
                // exclude the queue family we've already found and filter by transfer operation support
                .filter(|(i, q)| *i != render_family && q.queue_flags.contains(QueueFlags::TRANSFER))
                // some drivers expose a queue that only supports transfer operations (for this very purpose) which is preferable
                .max_by_key(|(_, q)| if !q.queue_flags.contains(QueueFlags::GRAPHICS) { 1 } else { 0 })
                .map(|(i, _)| i);
            
            Some(ChoosePhysicalDeviceReturn {
                physical_device: p,
                render_queue_family_index: render_family,
                transfer_queue_family_index: transfer_family.unwrap_or(render_family)
            })
        })
        // preference of device type
        .max_by_key(
            |ChoosePhysicalDeviceReturn {
                 physical_device, ..
             }| match physical_device.properties().device_type {
                vk::PhysicalDeviceType::DiscreteGpu => 4,
                vk::PhysicalDeviceType::IntegratedGpu => 3,
                vk::PhysicalDeviceType::VirtualGpu => 2,
                vk::PhysicalDeviceType::Cpu => 1,
                vk::PhysicalDeviceType::Other => 0,
                _ne => 0,
            },
        );

    chosen_device.with_context(|| {
            format!(
                "could not find a suitable vulkan physical device. requirements:\n
            \t- must support minimum vulkan version {}.{}\n
            \t- must contain queue family supporting graphics, transfer and surface operations\n
            \t- must support device extensions: {:?}\n
            \t- must support device features: {:?}",
                VULKAN_VER_MAJ, VULKAN_VER_MIN, required_extensions, required_features
            )
        })
}

/// Contains Vulkan resources and methods to manage rendering
pub struct RenderManager {
    entry: ash::Entry,
    instance: Arc<Instance>,
    debug_callback: vk::DebugUtilsMessengerEXT,

    physical_device: PhysicalDevice,

    window: Arc<Window>,
    surface: Surface,
    is_srgb_framebuffer: bool,

    /*
    device: Arc<Device>,
    render_queue: Arc<Queue>,
    _transfer_queue: Arc<Queue>,
    _debug_callback: Option<DebugUtilsMessenger>,

    window: Arc<Window>,
    _surface: Arc<Surface>,
    swapchain: Arc<Swapchain>,
    swapchain_image_views: Vec<Arc<ImageView<SwapchainImage>>>,
    is_srgb_framebuffer: bool,

    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer_allocator: StandardCommandBufferAllocator,
    descriptor_allocator: Arc<StandardDescriptorSetAllocator>,

    depth_buffer: Arc<ImageView<AttachmentImage>>,
    g_buffer_normal: Arc<ImageView<AttachmentImage>>,
    g_buffer_primitive_id: Arc<ImageView<AttachmentImage>>,

    viewport: Viewport,
    render_pass: Arc<RenderPass>,
    framebuffers: Vec<Arc<Framebuffer>>,
    clear_values: [Option<ClearValue>; ATTACHMENT_COUNT],
    */

    geometry_pass: GeometryPass,
    lighting_pass: LightingPass,
    //overlay_pass: OverlayPass,
    gui_pass: GuiRenderer,

    /// Some resources are duplicated `FRAMES_IN_FLIGHT` times in order to manipulate resources
    /// without conflicting with commands currently being processed. This variable indicates
    /// which index to will be next submitted to the GPU.
    next_frame: usize,
    /// Indicates that the swapchain needs to be recreated next frame
    recreate_swapchain: bool,
}

// Public functions

impl RenderManager {
    /// Initializes Vulkan resources. If renderer fails to initiver_minoralize, returns a string explanation.
    pub fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let entry = ash::Entry::linked();

        // create instance
        let api_version = ApiVersion {
            major: VULKAN_VER_MAJ,
            minor: VULKAN_VER_MIN,
        };
        let instance = Arc::new(Instance::new(
            &entry,
            api_version,
            ENGINE_NAME,
            window.raw_display_handle(),
            ENABLE_VULKAN_VALIDATION,
            Vec::new(),
            Vec::new(),
        )?);
        info!(
            "created vulkan instance. api version = {}",
            instance.api_version()
        );

        // setup validation layer debug callback
        let debug_callback = if ENABLE_VULKAN_VALIDATION {
            Some(
                DebugCallback::new(&entry, instance.clone(), log_vulkan_debug_callback)
                    .context("creating vulkan debug callback")?,
            )
        } else {
            None
        };

        // create surface
        let surface = Surface::new(
            &entry,
            instance.clone(),
            window.raw_display_handle(),
            window.raw_window_handle(),
        )
        .context("creating vulkan surface")?;

        // choose physical device
        let physical_device = choose_physical_device_and_queue_families(&instance, &surface)?;

        /// TODO BRUH ///
        //
        // required instance extensions for platform surface rendering
        let mut instance_extensions = vulkano_win::required_extensions(&vulkan_library);
        let mut instance_layers: Vec<String> = Vec::new();

        // check for validation layer/debug callback support
        let enable_debug_callback = if ENABLE_VULKAN_VALIDATION {
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
            debug!("Vulkan validation layers disabled via config.rs");
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
        let device_extensions = required_device_extensions();
        debug!("required vulkan device extensions: {:?}", device_extensions);

        // print available physical devices
        debug!("available Vulkan physical devices:");
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
                enabled_features: required_features(),
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

        // swapchain
        let (swapchain, swapchain_images) = create_swapchain(
            device.clone(),
            physical_device.clone(),
            surface.clone(),
            &window,
        )?;
        debug!(
            "initial swapchain image size = {:?}",
            swapchain_images[0].dimensions()
        );
        let is_srgb_framebuffer = is_srgb_framebuffer(swapchain_images[0].clone());
        let swapchain_image_views = swapchain_images
            .iter()
            .map(|image| ImageView::new_default(image.clone()))
            .collect::<Result<Vec<_>, _>>()
            .context("creating swapchain image views")?;

        // dynamic viewport
        let framebuffer_dimensions = swapchain_images[0].dimensions().width_height();
        let viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [
                framebuffer_dimensions[0] as f32,
                framebuffer_dimensions[1] as f32,
            ],
            depth_range: 0.0..1.0,
        };

        // create render_pass
        let multisample = SampleCount::Sample1;
        let render_pass = create_render_pass(device.clone(), &swapchain_image_views, multisample)?;

        // describe subpasses
        let subpass_gbuffer = Subpass::from(
            render_pass.clone(),
            render_pass_indices::SUBPASS_GBUFFER as u32,
        )
        .ok_or(anyhow!(
            "render pass does not contain subpass at index {}",
            render_pass_indices::SUBPASS_GBUFFER
        ))?;
        let subpass_swapchain = Subpass::from(
            render_pass.clone(),
            render_pass_indices::SUBPASS_SWAPCHAIN as u32,
        )
        .ok_or(anyhow!(
            "render pass does not contain subpass at index {}",
            render_pass_indices::SUBPASS_SWAPCHAIN
        ))?;

        // allocators
        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
        let command_buffer_allocator = StandardCommandBufferAllocator::new(
            device.clone(),
            StandardCommandBufferAllocatorCreateInfo {
                primary_buffer_count: PRE_ALLOCATE_PRIMARY_COMMAND_BUFFERS,
                secondary_buffer_count: PRE_ALLOCATE_SECONDARY_COMMAND_BUFFERS,
                ..StandardCommandBufferAllocatorCreateInfo::default()
            },
        );
        let descriptor_allocator = Arc::new(StandardDescriptorSetAllocator::new(device.clone()));

        // depth buffer
        let depth_buffer = create_depth_buffer(&memory_allocator, framebuffer_dimensions)?;

        // g-buffers
        let g_buffer_normal = create_g_buffer_normals(&memory_allocator, framebuffer_dimensions)?;
        let g_buffer_primitive_id =
            create_g_buffer_primitive_ids(&memory_allocator, framebuffer_dimensions)?;

        // create framebuffers
        let framebuffers = create_framebuffers(
            render_pass.clone(),
            &swapchain_image_views,
            g_buffer_normal.clone(),
            g_buffer_primitive_id.clone(),
            depth_buffer.clone(),
        )?;
        let mut clear_values: [Option<ClearValue>; ATTACHMENT_COUNT] = Default::default();
        clear_values[render_pass_indices::ATTACHMENT_SWAPCHAIN] = Some([0., 0., 0., 1.].into());
        clear_values[render_pass_indices::ATTACHMENT_NORMAL] = Some([0.; 4].into());
        clear_values[render_pass_indices::ATTACHMENT_PRIMITIVE_ID] =
            Some([primitive_codes::INVALID; 4].into());
        clear_values[render_pass_indices::ATTACHMENT_DEPTH_BUFFER] = Some(ClearValue::Depth(1.));

        // init lighting pass
        let lighting_pass = LightingPass::new(
            device.clone(),
            descriptor_allocator.clone(),
            g_buffer_normal.clone(),
            g_buffer_primitive_id.clone(),
            subpass_swapchain.clone(),
        )?;

        // init geometry pass
        let geometry_pass = GeometryPass::new(
            device.clone(),
            memory_allocator.clone(),
            descriptor_allocator.clone(),
            subpass_gbuffer,
        )?;

        // init overlay pass
        //let overlay_pass =
        //    OverlayPass::new(device.clone(), &memory_allocator, subpass_swapchain.clone())?;

        // init gui renderer
        let gui_pass = GuiRenderer::new(
            device.clone(),
            memory_allocator.clone(),
            render_queue.clone(),
            subpass_swapchain.clone(),
        )?;

        // create futures used for frame synchronization
        let future_previous_frame = Some(sync::now(device.clone()).boxed());

        Ok(Self {
            device,
            render_queue,
            _transfer_queue: transfer_queue,
            _debug_callback: debug_callback,

            window,
            _surface: surface,
            swapchain,
            swapchain_image_views,
            is_srgb_framebuffer,

            memory_allocator,
            command_buffer_allocator,
            descriptor_allocator,

            depth_buffer,
            g_buffer_normal,
            g_buffer_primitive_id,

            viewport,
            render_pass,
            framebuffers,
            clear_values,

            geometry_pass,
            lighting_pass,
            //overlay_pass,
            gui_pass,

            next_frame: 0,
            future_previous_frame,
            recreate_swapchain: false,
        })
    }

    pub fn update_object_buffers(
        &mut self,
        object_collection: &ObjectCollection,
        object_delta: ObjectsDelta,
    ) -> anyhow::Result<()> {
        self.geometry_pass
            .update_object_buffers(object_collection, object_delta)
    }

    pub fn update_gui_textures(
        &mut self,
        textures_delta_vec: Vec<TexturesDelta>,
    ) -> anyhow::Result<()> {
        self.wait_and_unlock_previous_frame();

        self.future_previous_frame = Some(
            self.gui_pass.update_textures(
                self.future_previous_frame
                    .take()
                    .unwrap_or(sync::now(self.device.clone()).boxed()), // should never be None anyway...
                &self.command_buffer_allocator,
                &self.descriptor_allocator,
                textures_delta_vec,
                self.render_queue.clone(),
            )?,
        );
        Ok(())
    }

    /// Submits Vulkan commands for rendering a frame.
    pub fn render_frame(
        &mut self,
        window_resize: bool,
        gui: &mut Gui,
        camera: &mut Camera,
    ) -> anyhow::Result<()> {
        let camera_buffer = create_camera_buffer(
            &self.memory_allocator,
            CameraUniformBuffer::from_camera(camera, self.viewport.dimensions),
        )?;

        self.wait_and_unlock_previous_frame();

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
        // 'suboptimal' indicates that the swapchain image will still work but may not be displayed correctly
        // we'll render the frame anyway hehe
        if suboptimal {
            debug!(
                "suboptimal swapchain image {}, rendering anyway...",
                swapchain_index
            );
            self.recreate_swapchain = true;
        }

        // record command buffer
        let mut builder = command_buffer::AutoCommandBufferBuilder::primary(
            &self.command_buffer_allocator,
            self.render_queue.queue_family_index(),
            command_buffer::CommandBufferUsage::OneTimeSubmit,
        )
        .context("beginning primary command buffer")?;

        // begin render pass
        builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: self.clear_values.into(),
                    ..RenderPassBeginInfo::framebuffer(
                        self.framebuffers[swapchain_index as usize].clone(),
                    )
                },
                SubpassContents::Inline,
            )
            .context("recording begin render pass command")?;

        // geometry ray-marching pass (outputs to g-buffer)
        self.geometry_pass.record_commands(
            &mut builder,
            self.viewport.clone(),
            camera_buffer.clone(),
        )?;

        builder
            .next_subpass(SubpassContents::Inline)
            .context("recording next subpass command")?;

        // execute deferred lighting pass (reads from g-buffer)
        self.lighting_pass.record_commands(
            &mut builder,
            self.viewport.clone(),
            camera_buffer.clone(),
        )?;

        // draw editor overlay
        //self.overlay_pass
        //    .record_commands(&mut builder, camera, self.viewport.clone())?;

        // render gui
        self.gui_pass.record_commands(
            &mut builder,
            gui,
            self.is_srgb_framebuffer,
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

        self.next_frame = (self.next_frame + 1) % FRAMES_IN_FLIGHT;
        Ok(())
    }
}

// Private functions

impl RenderManager {
    /// Recreates the swapchain, g-buffers and assiciated descriptor sets, then unsets `recreate_swapchain` trigger.
    fn recreate_swapchain(&mut self) -> anyhow::Result<()> {
        let new_size: [u32; 2] = self.window.inner_size().into();

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

        let resolution = swapchain_images[0].dimensions().width_height();
        self.viewport.dimensions = [resolution[0] as f32, resolution[1] as f32];

        // depth buffer
        self.depth_buffer = create_depth_buffer(&self.memory_allocator, resolution)?;

        // g-buffers
        self.g_buffer_normal = create_g_buffer_normals(&self.memory_allocator, resolution)?;
        self.g_buffer_primitive_id =
            create_g_buffer_primitive_ids(&self.memory_allocator, resolution)?;

        self.framebuffers = create_framebuffers(
            self.render_pass.clone(),
            &self.swapchain_image_views,
            self.g_buffer_normal.clone(),
            self.g_buffer_primitive_id.clone(),
            self.depth_buffer.clone(),
        )?;

        self.lighting_pass.update_g_buffers(
            &self.descriptor_allocator,
            self.g_buffer_normal.clone(),
            self.g_buffer_primitive_id.clone(),
        )?;

        self.is_srgb_framebuffer = is_srgb_framebuffer(swapchain_images[0].clone());

        self.recreate_swapchain = false;
        Ok(())
    }

    /// Checks for submission finish and free locks on gpu resources
    fn wait_and_unlock_previous_frame(&mut self) {
        if let Some(future_previous_frame) = self.future_previous_frame.as_mut() {
            future_previous_frame.cleanup_finished();
        }
    }
}

fn create_depth_buffer(
    memory_allocator: &StandardMemoryAllocator,
    dimensions: [u32; 2],
) -> Result<Arc<ImageView<AttachmentImage>>, anyhow::Error> {
    ImageView::new_default(
        AttachmentImage::transient(memory_allocator, dimensions, FORMAT_DEPTH_BUFFER)
            .context("creating depth buffer image")?,
    )
    .context("creating depth buffer image view")
}

fn create_g_buffer_normals(
    memory_allocator: &StandardMemoryAllocator,
    dimensions: [u32; 2],
) -> Result<Arc<ImageView<AttachmentImage>>, anyhow::Error> {
    ImageView::new_default(
        AttachmentImage::with_usage(
            memory_allocator,
            dimensions,
            FORMAT_G_BUFFER_NORMAL,
            ImageUsage {
                transient_attachment: true,
                input_attachment: true,
                ..ImageUsage::empty()
            },
        )
        .context("creating normal g-buffer image")?,
    )
    .context("creating normal g-buffer image view")
}

fn create_g_buffer_primitive_ids(
    memory_allocator: &StandardMemoryAllocator,
    dimensions: [u32; 2],
) -> Result<Arc<ImageView<AttachmentImage>>, anyhow::Error> {
    ImageView::new_default(
        AttachmentImage::with_usage(
            memory_allocator,
            dimensions,
            FORMAT_G_BUFFER_PRIMITIVE_ID,
            ImageUsage {
                transient_attachment: false,
                input_attachment: true,
                ..ImageUsage::empty()
            },
        )
        .context("creating g-buffer")?,
    )
    .context("creating g-buffer image view")
}
