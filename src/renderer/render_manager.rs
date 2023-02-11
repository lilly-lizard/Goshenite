use super::{
    config_renderer::{
        ENABLE_VULKAN_VALIDATION, FORMAT_DEPTH_BUFFER, FORMAT_G_BUFFER_NORMAL,
        FORMAT_G_BUFFER_PRIMITIVE_ID, FRAMES_IN_FLIGHT, VULKAN_VER_MAJ, VULKAN_VER_MIN,
    },
    shader_interfaces::{
        primitive_op_buffer::primitive_codes, uniform_buffers::CameraUniformBuffer,
    },
};
use crate::{
    config::ENGINE_NAME,
    engine::object::{object_collection::ObjectCollection, objects_delta::ObjectsDelta},
    user_interface::{camera::Camera, gui::Gui},
};
use anyhow::Context;
use ash::{vk::{
    self, DebugUtilsMessageSeverityFlagsEXT, PhysicalDeviceVulkan12Features, QueueFlags,
}, Entry};
use bort::{
    debug_callback::DebugCallback,
    instance::{ApiVersion, Instance},
    physical_device::PhysicalDevice,
    surface::Surface,
};
use egui::TexturesDelta;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
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

fn required_device_extensions() -> [&'static str; 2] {
    [
        "VK_KHR_swapchain",
        "VK_EXT_descriptor_indexing",
    ]
}

/// Make sure to update `required_features_1_2` too!
fn supports_required_features_1_2(supported_features: PhysicalDeviceVulkan12Features) -> bool {
    supported_features.descriptor_indexing == vk::TRUE
        && supported_features.runtime_descriptor_array == vk::TRUE
        && supported_features.descriptor_binding_variable_descriptor_count == vk::TRUE
        && supported_features.shader_storage_buffer_array_non_uniform_indexing == vk::TRUE
        && supported_features.descriptor_binding_partially_bound == vk::TRUE
}
/// Make sure to update `supports_required_features_1_2` too!
fn required_features_1_2() -> PhysicalDeviceVulkan12Features {
    PhysicalDeviceVulkan12Features {
        descriptor_indexing: vk::TRUE,
        runtime_descriptor_array: vk::TRUE,
        descriptor_binding_variable_descriptor_count: vk::TRUE,
        shader_storage_buffer_array_non_uniform_indexing: vk::TRUE,
        descriptor_binding_partially_bound: vk::TRUE,
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
        .map(|&handle| PhysicalDevice::new(instance, handle))
        .collect::<Result<Vec<_>, _>>()?;

    // print available physical devices
    debug!("available vulkan physical devices:");
    for pd in &p_devices
    {
        debug!("\t{}", pd.name());
    }

    let required_extensions = required_device_extensions();
    let required_features = required_features_1_2();
    trace!(
        "required physical device extensions = {:?}",
        required_extensions
    );
    trace!(
        "required physical device features = {:?}",
        required_features
    );

    let chosen_device = p_devices
        .into_iter()
        // filter for supported api version
        .filter(|p| p.supports_api_ver(instance.api_version()))
        // filter for required device extensionssupports_extension
        .filter(|p| p.supports_extensions(required_extensions.into_iter()))
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
                            .get_physical_device_surface_support(&p, i as u32)
                            .unwrap_or(false)
                });
            let render_family = match render_family {
                Some(x) => x,
                None => {
                    debug!("no suitable queue family index found for physical device {}", p.name());
                    return None;
                },
            };

            // check requried device features support
            let supported_features = instance.physical_device_features_1_2(p.handle()).expect("instance should have been created for vulkan 1.2");
            if supports_required_features_1_2(supported_features) {
                trace!(
                    "physical device {} doesn't support required features. supported features: {:?}",
                    p.name(),
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
                vk::PhysicalDeviceType::DISCRETE_GPU => 4,
                vk::PhysicalDeviceType::INTEGRATED_GPU => 3,
                vk::PhysicalDeviceType::VIRTUAL_GPU => 2,
                vk::PhysicalDeviceType::CPU => 1,
                vk::PhysicalDeviceType::OTHER => 0,
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

    //geometry_pass: GeometryPass,
    //lighting_pass: LightingPass,
    //overlay_pass: OverlayPass,
    //gui_pass: GuiRenderer,

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
        let entry = unsafe { Entry::load() }.context("loading dynamic library. please install vulkan on your system...")?;

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
            "created vulkan instance. api version = {:?}",
            instance.api_version()
        );

        // setup validation layer debug callback
        let debug_callback = if ENABLE_VULKAN_VALIDATION {
            match DebugCallback::new(&entry, instance.clone(), Some(log_vulkan_debug_callback)) {
                Ok(x) => {
                    info!("enabling vulkan validation layers and debug callback");
                    Some(x)
                },
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
            "Using Vulkan device: {} (type: {:?})",
            physical_device.name(),
            physical_device.properties().device_type,
        );
        debug!("render queue family index = {}", render_queue_family_index);
        debug!("transfer queue family index = {}", transfer_queue_family_index);

        todo!();
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
