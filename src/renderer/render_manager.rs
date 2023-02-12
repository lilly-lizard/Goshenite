use super::config_renderer::{ENABLE_VULKAN_VALIDATION, VULKAN_VER_MAJ, VULKAN_VER_MIN};
use crate::{
    config::ENGINE_NAME,
    engine::object::{object_collection::ObjectCollection, objects_delta::ObjectsDelta},
    renderer::vulkan_init::{
        choose_physical_device_and_queue_families, create_device_and_queues, create_swapchain,
        ChoosePhysicalDeviceReturn, CreateDeviceAndQueuesReturn,
    },
    user_interface::{camera::Camera, gui::Gui},
};
use anyhow::Context;
use ash::{vk, Entry};
use bort::{
    common::is_format_srgb,
    debug_callback::DebugCallback,
    device::Device,
    image::SwapchainImage,
    instance::{ApiVersion, Instance},
    physical_device::PhysicalDevice,
    queue::Queue,
    surface::Surface,
    swapchain::Swapchain,
};
use egui::TexturesDelta;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::{borrow::Cow, ffi::CStr, sync::Arc};
use winit::window::Window;

/// Contains Vulkan resources and methods to manage rendering
pub struct RenderManager {
    entry: ash::Entry,
    instance: Arc<Instance>,
    debug_callback: Option<DebugCallback>,

    window: Arc<Window>,
    surface: Surface,
    swapchain: Arc<Swapchain>,
    swapchain_images: Vec<SwapchainImage>,
    is_swapchain_srgb: bool,

    physical_device: PhysicalDevice,
    device: Arc<Device>,
    render_queue: Queue,
    transfer_queue: Option<Queue>,

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
        let entry = unsafe { Entry::load() }
            .context("loading dynamic library. please install vulkan on your system...")?;

        // create instance
        let api_version = ApiVersion::new(VULKAN_VER_MAJ, VULKAN_VER_MIN);
        let instance = Arc::new(Instance::new(
            &entry,
            api_version,
            ENGINE_NAME,
            window.raw_display_handle(),
            ENABLE_VULKAN_VALIDATION,
            [],
            [],
        )?);
        info!(
            "created vulkan instance. api version = {:?}",
            instance.api_version()
        );

        // setup validation layer debug callback
        let debug_callback = if ENABLE_VULKAN_VALIDATION {
            match DebugCallback::new(&entry, instance.clone(), Some(log_vulkan_debug_callback)) {
                Ok(x) => {
                    info!("enabling vulkan validation layers and debug callback");
                    Some(x)
                }
                Err(e) => {
                    warn!("validation layer debug callback requested but cannot be setup due to: {:?}", e);
                    None
                }
            }
        } else {
            debug!("vulkan validation layers disabled");
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

        // choose physical device and queue families
        let ChoosePhysicalDeviceReturn {
            physical_device,
            render_queue_family_index,
            transfer_queue_family_index,
        } = choose_physical_device_and_queue_families(&instance, &surface)?;
        info!(
            "using vulkan physical device: {} (type: {:?})",
            physical_device.name(),
            physical_device.properties().device_type,
        );
        debug!("render queue family index = {}", render_queue_family_index);
        debug!(
            "transfer queue family index = {}",
            transfer_queue_family_index
        );

        // create device and queues
        let CreateDeviceAndQueuesReturn {
            device,
            render_queue,
            transfer_queue,
        } = create_device_and_queues(
            &instance,
            &physical_device,
            render_queue_family_index,
            transfer_queue_family_index,
        )?;

        // create swapchain
        let swapchain = Arc::new(create_swapchain(
            &instance,
            &device,
            &surface,
            &window,
            &physical_device,
        )?);
        debug!(
            "swapchain surface format = {:?}",
            swapchain.surface_format()
        );
        debug!("swapchain present mode = {:?}", swapchain.present_mode());
        debug!(
            "swapchain composite alpha = {:?}",
            swapchain.composite_alpha()
        );
        let is_swapchain_srgb = is_format_srgb(swapchain.surface_format().format);

        // create swapchain images
        let swapchain_images = SwapchainImage::from_swapchain(device.clone(), swapchain.clone())?;

        Ok(Self {
            entry,
            instance,
            debug_callback,

            window,
            surface,
            swapchain,
            swapchain_images,
            is_swapchain_srgb,

            physical_device,
            device,
            render_queue,
            transfer_queue,

            next_frame: 0,
            recreate_swapchain: false,
        })
    }

    pub fn update_object_buffers(
        &mut self,
        object_collection: &ObjectCollection,
        object_delta: ObjectsDelta,
    ) -> anyhow::Result<()> {
        todo!();
    }

    pub fn update_gui_textures(
        &mut self,
        textures_delta_vec: Vec<TexturesDelta>,
    ) -> anyhow::Result<()> {
        todo!();
    }

    /// Submits Vulkan commands for rendering a frame.
    pub fn render_frame(
        &mut self,
        window_resize: bool,
        gui: &mut Gui,
        camera: &mut Camera,
    ) -> anyhow::Result<()> {
        todo!();
    }
}

// Private functions

impl RenderManager {
    /// Recreates the swapchain, g-buffers and assiciated descriptor sets, then unsets `recreate_swapchain` trigger.
    fn recreate_swapchain(&mut self) -> anyhow::Result<()> {
        todo!();
    }
}

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
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            error!("Vulkan [{:?}]:\n{}", message_type, message);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            warn!("Vulkan [{:?}]:\n{}", message_type, message);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => {
            info!("Vulkan [{:?}]:\n{}", message_type, message);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => {
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
