use super::renderer_config::SHADER_ENTRY_POINT;
use crate::renderer::renderer_config::{G_BUFFER_FORMAT_NORMAL, G_BUFFER_FORMAT_PRIMITIVE_ID};
use anyhow::{bail, ensure, Context};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::{fmt, sync::Arc};
use vulkano::{
    device::{
        self,
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceExtensions,
    },
    format::{Format, NumericType},
    image::{
        view::ImageView, AttachmentImage, ImageAccess, ImageUsage, ImageViewAbstract, SampleCount,
        SwapchainImage,
    },
    instance::debug::{
        DebugUtilsMessageSeverity, DebugUtilsMessageType, DebugUtilsMessenger,
        DebugUtilsMessengerCreateInfo,
    },
    instance::{Instance, InstanceExtensions},
    memory::allocator::MemoryAllocator,
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass},
    shader::{ShaderCreationError, ShaderModule},
    swapchain::{self, Surface, Swapchain},
    VulkanLibrary,
};
use winit::window::Window;

use super::renderer_config::{VULKAN_VER_MAJ, VULKAN_VER_MIN};

pub fn required_device_extensions() -> DeviceExtensions {
    DeviceExtensions {
        khr_swapchain: true,
        ext_descriptor_indexing: true,
        ..DeviceExtensions::empty()
    }
}

pub fn required_features() -> device::Features {
    device::Features {
        descriptor_indexing: true,
        runtime_descriptor_array: true,
        descriptor_binding_variable_descriptor_count: true,
        shader_storage_buffer_array_non_uniform_indexing: true,
        descriptor_binding_partially_bound: true,
        ..device::Features::empty()
    }
}

pub mod render_pass_indices {
    pub const ATTACHMENT_SWAPCHAIN: usize = 0;
    pub const ATTACHMENT_NORMAL: usize = 1;
    pub const ATTACHMENT_PRIMITIVE_ID: usize = 2;
    pub const SUBPASS_GBUFFER: usize = 0;
    pub const SUBPASS_SWAPCHAIN: usize = 1;
}

pub type QueueFamilyIndex = u32;

/// Checks for VK_EXT_debug_utils support and presence khronos validation layers
/// If both can be enabled, adds them to provided extension and layer lists
pub fn add_debug_validation(
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

pub fn setup_debug_callback(instance: Arc<Instance>) -> Option<DebugUtilsMessenger> {
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

pub fn choose_physical_device(
    instance: Arc<Instance>,
    device_extensions: &DeviceExtensions,
    surface: &Arc<Surface>,
) -> anyhow::Result<ChoosePhysicalDeviceReturn> {
    let required_features = required_features();
    instance
        .enumerate_physical_devices()
        .context("enumerating physical devices")?
        // filter for vulkan version support
        .filter(|p| {
            p.api_version() >= vulkano::Version::major_minor(VULKAN_VER_MAJ, VULKAN_VER_MIN)
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
                        && q.queue_flags.transfer
                        && p.surface_support(i as u32, surface).unwrap_or(false)
                });

            let supported_features = p.supported_features();
            if !supported_features.contains(&required_features) {
                // device doesn't support features
                let missing_features = required_features.difference(supported_features);
                debug!(
                    "physical device {} doesn't support the following required features: {:?}",
                    p.properties().device_name,
                    &missing_features
                );
                return None;
            }

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
                    .max_by_key(|(_, q)| if !q.queue_flags.graphics { 1 } else { 0 })
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
        .with_context(|| {
            format!(
                "could not find a suitable vulkan physical device. requirements:\n
            \t- must support minimum vulkan version {}.{}\n
            \t- must contain queue family supporting graphics, transfer and surface operations\n
            \t- must support device extensions: {:?}",
                VULKAN_VER_MAJ, VULKAN_VER_MIN, device_extensions
            )
        })
}
pub struct ChoosePhysicalDeviceReturn {
    pub physical_device: Arc<PhysicalDevice>,
    pub render_queue_family: QueueFamilyIndex,
    pub transfer_queue_family: QueueFamilyIndex,
}

pub fn create_swapchain(
    device: Arc<Device>,
    physical_device: Arc<PhysicalDevice>,
    surface: Arc<Surface>,
    window: &Window,
) -> anyhow::Result<(Arc<Swapchain>, Vec<Arc<SwapchainImage>>)> {
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
            image_extent: window.inner_size().into(),
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

pub fn is_srgb_framebuffer(swapchain_image: Arc<SwapchainImage>) -> bool {
    swapchain_image
        .format()
        .type_color()
        .unwrap_or(NumericType::UNORM)
        == NumericType::SRGB
}

/// Creates the render target for the scene render. _Note that the value of `access_queue` isn't actually used
/// in the vulkan image creation create info._
pub fn create_g_buffer(
    memory_allocator: &impl MemoryAllocator,
    size: [u32; 2],
    format: Format,
) -> anyhow::Result<Arc<ImageView<AttachmentImage>>> {
    ImageView::new_default(
        AttachmentImage::with_usage(
            memory_allocator,
            size,
            format,
            ImageUsage {
                transient_attachment: true,
                input_attachment: true,
                ..ImageUsage::empty()
            },
        )
        .context("creating g-buffer")?,
    )
    .context("creating g-buffer image view")
}

pub fn create_render_pass(
    device: Arc<Device>,
    swapchain_image_views: &Vec<Arc<ImageView<SwapchainImage>>>,
    sample_count: SampleCount,
) -> anyhow::Result<Arc<RenderPass>> {
    ensure!(
        swapchain_image_views.len() >= 1,
        "no swapchain images provided to create render pass"
    );
    let swapchain_image = &swapchain_image_views[0].image();

    // ensure indices match macro below
    let msg = "render_pass_indices don't match macro array below!";
    debug_assert!(render_pass_indices::ATTACHMENT_SWAPCHAIN == 0, "{}", msg);
    debug_assert!(render_pass_indices::ATTACHMENT_NORMAL == 1, "{}", msg);
    debug_assert!(render_pass_indices::ATTACHMENT_PRIMITIVE_ID == 2, "{}", msg);
    debug_assert!(render_pass_indices::SUBPASS_GBUFFER == 0, "{}", msg);
    debug_assert!(render_pass_indices::SUBPASS_SWAPCHAIN == 1, "{}", msg);

    // create render pass boilerplate and object
    vulkano::ordered_passes_renderpass!(device,
        attachments: {
            // swapchain image
            swapchain: {
                load: Clear,
                store: Store,
                format: swapchain_image.format(),
                samples: sample_count,
            },
            // normal g-buffer
            g_buffer_normal: {
                load: Clear,
                store: DontCare,
                format: G_BUFFER_FORMAT_NORMAL,
                samples: sample_count,
            },
            // primitive-id g-buffer
            g_buffer_primitive_id: {
                load: Clear,
                store: DontCare,
                format: G_BUFFER_FORMAT_PRIMITIVE_ID,
                samples: sample_count,
            }
        },
        passes: [
            // gbuffer subpass
            {
                color: [g_buffer_normal, g_buffer_primitive_id],
                depth_stencil: {},
                input: []
            },
            // swapchain subpass
            {
                color: [swapchain],
                depth_stencil: {},
                input: [g_buffer_normal, g_buffer_primitive_id]
            }
        ]
    )
    .context("creating vulkan render pass")
}

pub fn create_framebuffers(
    render_pass: Arc<RenderPass>,
    swapchain_image_views: &Vec<Arc<ImageView<SwapchainImage>>>,
    g_buffer_normal: Arc<ImageView<AttachmentImage>>,
    g_buffer_primitive_id: Arc<ImageView<AttachmentImage>>,
) -> anyhow::Result<Vec<Arc<Framebuffer>>> {
    // 1 for each swapchain image
    swapchain_image_views
        .iter()
        .map(|image_view| {
            let mut attachments: Vec<Arc<dyn ImageViewAbstract>> = Vec::default();
            attachments.insert(
                render_pass_indices::ATTACHMENT_SWAPCHAIN,
                image_view.clone(),
            );
            attachments.insert(
                render_pass_indices::ATTACHMENT_NORMAL,
                g_buffer_normal.clone(),
            );
            attachments.insert(
                render_pass_indices::ATTACHMENT_PRIMITIVE_ID,
                g_buffer_primitive_id.clone(),
            );
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments,
                    ..Default::default()
                },
            )
            .context("creating vulkan framebuffer")
        })
        .collect::<anyhow::Result<Vec<_>>>()
}

/// Creates a Vulkan shader module given a spirv path (relative to crate root)
pub fn create_shader_module(
    device: Arc<Device>,
    spirv_path: &str,
) -> Result<Arc<ShaderModule>, CreateShaderError> {
    // read spirv bytes
    let bytes = match std::fs::read(spirv_path) {
        Ok(x) => x,
        Err(e) => {
            return Err(CreateShaderError::IOError {
                e,
                path: spirv_path.to_owned(),
            })
        }
    };
    // create shader module
    // todo conv to &[u32] and use from_words (guarentees 4 byte multiple)
    match unsafe { ShaderModule::from_bytes(device.clone(), bytes.as_slice()) } {
        Ok(x) => Ok(x),
        Err(e) => {
            return Err(CreateShaderError::ShaderCreationError {
                e,
                path: spirv_path.to_owned(),
            })
        }
    }
}

/// This mod just makes the module path unique for debug callbacks in the log
pub mod vulkan_callback {
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

// ~~~ Errors ~~~

/// Errors encountered when preparing shader
#[derive(Debug)]
pub enum CreateShaderError {
    /// Shader SPIR-V read failed. The string should contain the shader file path.
    IOError { e: std::io::Error, path: String },
    /// Shader module creation failed. The string should contain the shader file path.
    ShaderCreationError {
        e: ShaderCreationError,
        path: String,
    },
    /// Shader is missing entry point `main`. String should contain shader path
    MissingEntryPoint(String),
}
impl fmt::Display for CreateShaderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Self::IOError { e, path } => write!(f, "failed to read shader file {}: {}", path, e),
            Self::ShaderCreationError { e, path } => {
                write!(f, "failed to create shader module from {}: {}", path, e)
            }
            Self::MissingEntryPoint(path) => {
                write!(
                    f,
                    "shader {} is missing entry point `{}`",
                    path, SHADER_ENTRY_POINT
                )
            }
        }
    }
}
impl std::error::Error for CreateShaderError {}

/// Errors encountered when creating a descriptor set
#[derive(Debug)]
pub enum CreateDescriptorSetError {
    /// Descriptor set index not found in the pipeline layout
    InvalidDescriptorSetIndex {
        /// Descriptor set index
        index: usize,
        /// A shader where this descriptor set is expected to be found, to assist with debugging
        shader_path: &'static str,
    },
}
impl std::error::Error for CreateDescriptorSetError {}
impl fmt::Display for CreateDescriptorSetError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InvalidDescriptorSetIndex { index, shader_path } => {
                write!(
                    f,
                    "descriptor set index {} not found in pipeline layout. possibly relavent shader {}",
                    index, shader_path
                )
            }
        }
    }
}
