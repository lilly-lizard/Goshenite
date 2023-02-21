use super::{
    config_renderer::{ENABLE_VULKAN_VALIDATION, VULKAN_VER_MAJ, VULKAN_VER_MIN},
    lighting_pass::LightingPass,
};
use crate::{
    config::ENGINE_NAME,
    engine::object::{object_collection::ObjectCollection, objects_delta::ObjectsDelta},
    renderer::{
        shader_interfaces::uniform_buffers::CameraUniformBuffer,
        vulkan_init::{
            choose_physical_device_and_queue_families, create_camera_ubo, create_clear_values,
            create_depth_buffer, create_device_and_queues, create_framebuffers,
            create_normal_buffer, create_primitive_id_buffer, create_render_pass, create_swapchain,
            create_swapchain_images, render_pass_indices, ChoosePhysicalDeviceReturn,
            CreateDeviceAndQueuesReturn,
        },
    },
    user_interface::{camera::Camera, gui::Gui},
};
use anyhow::Context;
use ash::{vk, Entry};
use bort::{
    buffer::Buffer,
    common::is_format_srgb,
    debug_callback::DebugCallback,
    descriptor_set::DescriptorSet,
    device::Device,
    framebuffer::Framebuffer,
    image::Image,
    image_view::ImageView,
    instance::{ApiVersion, Instance},
    memory::MemoryAllocator,
    queue::Queue,
    render_pass::RenderPass,
    surface::Surface,
    swapchain::Swapchain,
    swapchain_image::SwapchainImage,
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

    device: Arc<Device>,
    render_queue: Queue,
    transfer_queue: Option<Queue>,

    memory_allocator: Arc<MemoryAllocator>,

    window: Arc<Window>,
    surface: Arc<Surface>,
    swapchain: Arc<Swapchain>,
    swapchain_images: Vec<Arc<ImageView<SwapchainImage>>>,
    is_swapchain_srgb: bool,

    render_pass: Arc<RenderPass>,
    framebuffers: Vec<Arc<Framebuffer>>,
    clear_values: Vec<vk::ClearValue>,

    depth_buffer: Arc<ImageView<Image>>,
    normal_buffer: Arc<ImageView<Image>>,
    primitive_id_buffer: Arc<ImageView<Image>>,
    camera_ubo: Arc<Buffer>,

    lighting_pass: LightingPass,

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
        let surface = Arc::new(
            Surface::new(
                &entry,
                instance.clone(),
                window.raw_display_handle(),
                window.raw_window_handle(),
            )
            .context("creating vulkan surface")?,
        );

        // choose physical device and queue families
        let ChoosePhysicalDeviceReturn {
            physical_device,
            render_queue_family_index,
            transfer_queue_family_index,
        } = choose_physical_device_and_queue_families(instance.clone(), &surface)?;
        let physical_device = Arc::new(physical_device);
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
            physical_device.clone(),
            render_queue_family_index,
            transfer_queue_family_index,
        )?;

        // create memory allocator
        let memory_allocator = Arc::new(MemoryAllocator::new(device.clone())?);

        // create swapchain
        let swapchain = Arc::new(create_swapchain(
            device.clone(),
            surface.clone(),
            &window,
            &physical_device,
        )?);
        debug!(
            "swapchain surface format = {:?}",
            swapchain.properties().surface_format
        );
        debug!(
            "swapchain present mode = {:?}",
            swapchain.properties().present_mode
        );
        debug!(
            "swapchain composite alpha = {:?}",
            swapchain.properties().composite_alpha
        );
        let is_swapchain_srgb = is_format_srgb(swapchain.properties().surface_format.format);

        // create swapchain images
        let swapchain_images = create_swapchain_images(&swapchain)?;

        // create render pass
        let render_pass = Arc::new(create_render_pass(device.clone(), &swapchain)?);

        // create depth buffer
        let depth_buffer = Arc::new(create_depth_buffer(
            memory_allocator.clone(),
            swapchain.properties().dimensions(),
        )?);

        // create camera ubo
        let camera_ubo = Arc::new(create_camera_ubo(memory_allocator.clone())?);

        // create g-buffers
        let normal_buffer = Arc::new(create_normal_buffer(
            memory_allocator.clone(),
            swapchain.properties().dimensions(),
        )?);
        let primitive_id_buffer = Arc::new(create_primitive_id_buffer(
            memory_allocator.clone(),
            swapchain.properties().dimensions(),
        )?);

        // create framebuffers
        let framebuffers = create_framebuffers(
            &render_pass,
            &swapchain_images,
            &normal_buffer,
            &primitive_id_buffer,
            &depth_buffer,
        )?;
        let framebuffers = framebuffers.into_iter().map(|f| Arc::new(f)).collect();

        // clear values
        let clear_values = create_clear_values();

        let lighting_pass = LightingPass::new(
            device.clone(),
            &render_pass,
            render_pass_indices::SUBPASS_GBUFFER as u32,
            todo!(),
            normal_buffer.as_ref(),
            primitive_id_buffer.as_ref(),
        )?;

        Ok(Self {
            entry,
            instance,
            debug_callback,

            device,
            render_queue,
            transfer_queue,

            memory_allocator,

            window,
            surface,
            swapchain,
            swapchain_images,
            is_swapchain_srgb,

            render_pass,
            framebuffers,
            clear_values,

            depth_buffer,
            normal_buffer,
            primitive_id_buffer,
            camera_ubo,

            lighting_pass,

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
