use crate::renderer::config_renderer::ENABLE_VULKAN_VALIDATION;

use super::{
    config_renderer::{
        CPU_ACCESS_BUFFER_SIZE, FORMAT_NORMAL_BUFFER, FORMAT_PRIMITIVE_ID_BUFFER,
        MAX_FRAMES_IN_FLIGHT, MAX_VULKAN_VER, MIN_VULKAN_VER, SHADER_ENTRY_POINT,
    },
    shader_interfaces::{
        primitive_op_buffer::PRIMITIVE_ID_INVALID, uniform_buffers::CameraUniformBuffer,
    },
};
use anyhow::Context;
use ash::{prelude::VkResult, vk};
use bort_vk::{
    allocation_info_cpu_accessible, choose_composite_alpha, is_format_srgb, Buffer,
    BufferProperties, CommandBuffer, CommandPool, CommandPoolProperties, DebugCallback,
    DescriptorPool, DescriptorSet, DescriptorSetLayout, DescriptorSetLayoutBinding,
    DescriptorSetLayoutProperties, Device, DeviceOwned, Framebuffer, FramebufferProperties, Image,
    ImageDimensions, ImageProperties, ImageView, ImageViewAccess, ImageViewProperties, Instance,
    MemoryAllocator, PhysicalDevice, Queue, RenderPass, ShaderError, ShaderModule, ShaderStage,
    Subpass, Surface, Swapchain, SwapchainImage, SwapchainProperties,
};
use bort_vma::AllocationCreateInfo;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use raw_window_handle::HasRawDisplayHandle;
use std::{
    ffi::{CStr, CString},
    mem,
    sync::Arc,
};
use winit::window::Window;

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
pub fn create_entry() -> anyhow::Result<Arc<ash::Entry>> {
    let entry = unsafe { ash::Entry::load() }
        .context("loading vulkan dynamic library. please install vulkan on your system...")?;
    Ok(Arc::new(entry))
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub fn create_entry() -> anyhow::Result<Arc<ash::Entry>> {
    let entry = ash_molten::load();
    Ok(Arc::new(entry))
}

pub fn required_device_extensions() -> [&'static str; 2] {
    ["VK_KHR_swapchain", "VK_KHR_synchronization2"]
}

/// Make sure to update `required_features_1_2` too!
pub fn supports_required_features_1_2(
    _supported_features: vk::PhysicalDeviceVulkan12Features,
) -> bool {
    true
}

/// Make sure to update `supports_required_features_1_2` too!
pub fn required_features_1_2() -> vk::PhysicalDeviceVulkan12Features {
    vk::PhysicalDeviceVulkan12Features::default()
}

pub fn create_instance(entry: Arc<ash::Entry>, window: &Window) -> anyhow::Result<Arc<Instance>> {
    let mut layer_names = Vec::<&str>::new();

    let validation_layer_name = "VK_LAYER_KHRONOS_validation";

    if ENABLE_VULKAN_VALIDATION {
        let layer_properties = entry
            .enumerate_instance_layer_properties()
            .context("enumerating instance layer properties")?;

        for layer_prop in layer_properties {
            let layer_name = unsafe { CStr::from_ptr(layer_prop.layer_name.as_ptr()) }
                .to_str()
                .context("decoding installed layer names")?;

            if validation_layer_name == layer_name {
                debug!("enabling vulkan layer: VK_LAYER_KHRONOS_validation");
                layer_names.push(validation_layer_name);
                break;
            }
        }
    }

    let mut extension_names = Vec::<&str>::new();
    if ENABLE_VULKAN_VALIDATION {
        debug!("enabling instance extension: VK_EXT_debug_utils");
        extension_names.push("VK_EXT_debug_utils");
    };

    let instance = Arc::new(
        Instance::new(
            entry,
            MAX_VULKAN_VER,
            window.raw_display_handle(),
            layer_names,
            extension_names,
        )
        .context("creating vulkan instance")?,
    );

    info!(
        "created vulkan instance. max api version = {:?}",
        instance.max_api_version()
    );

    Ok(instance)
}

pub struct ChoosePhysicalDeviceReturn {
    pub physical_device: PhysicalDevice,
    pub render_queue_family_index: u32,
    pub transfer_queue_family_index: u32,
}

pub fn choose_physical_device_and_queue_families(
    instance: Arc<Instance>,
    surface: &Surface,
) -> anyhow::Result<ChoosePhysicalDeviceReturn> {
    let p_device_handles = unsafe { instance.inner().enumerate_physical_devices() }
        .context("enumerating physical devices")?;
    let p_devices: Vec<PhysicalDevice> = p_device_handles
        .iter()
        .map(|&handle| PhysicalDevice::new(instance.clone(), handle))
        .collect::<Result<Vec<_>, _>>()?;

    // print available physical devices
    debug!("available vulkan physical devices:");
    for pd in &p_devices {
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
        .filter(|p| p.supports_api_ver(MIN_VULKAN_VER))
        // filter for required device extensionssupports_extension
        .filter(|p| p.supports_extensions(required_extensions.into_iter()))
        // filter for queue support
        .filter_map(|p| check_physical_device_queue_support(p, surface, &instance))
        // prefer discrete gpus
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

    // converts Option to Result
    chosen_device.with_context(|| {
        format!(
            "could not find a suitable vulkan implimentation (device and driver). requirements:\n
            \t- must contain queue family supporting graphics, transfer and surface operations\n
            \t- must minimum api version: {:?}\n
            \t- must support device extensions: {:?}\n
            \t- must support device features: {:?}",
            MIN_VULKAN_VER, required_extensions, required_features
        )
    })
}

fn check_physical_device_queue_support(
    physical_device: PhysicalDevice,
    surface: &Surface,
    instance: &Instance,
) -> Option<ChoosePhysicalDeviceReturn> {
    // get queue family index for main queue
    let render_family = physical_device
        .queue_family_properties()
        .iter()
        // because we want the queue family index
        .enumerate()
        .position(|(queue_family_index, queue_family_properties)| {
            // must support our surface and essential operations
            let graphics_support = queue_family_properties
                .queue_flags
                .contains(vk::QueueFlags::GRAPHICS);
            let transfer_support = queue_family_properties
                .queue_flags
                .contains(vk::QueueFlags::TRANSFER);
            let surface_support = surface
                .get_physical_device_surface_support(&physical_device, queue_family_index as u32)
                .unwrap_or(false);
            graphics_support && transfer_support && surface_support
        });
    let render_family = match render_family {
        Some(x) => x as u32,
        None => {
            debug!(
                "no suitable queue family index found for physical device {}",
                physical_device.name()
            );
            return None;
        }
    };

    // check requried device features support
    let supported_features = instance
        .physical_device_features_1_2(physical_device.handle())
        .expect("instance should have been created for vulkan 1.2");
    if !supports_required_features_1_2(supported_features) {
        trace!(
            "physical device {} doesn't support required features. supported features: {:?}",
            physical_device.name(),
            supported_features
        );
        return None;
    }

    // attempt to find a different queue family that we can use for asynchronous transfer operations
    // e.g. uploading image/buffer data at same time as rendering
    let transfer_family = physical_device
        .queue_family_properties()
        .iter()
        .enumerate()
        // exclude the queue family we've already found and filter by transfer operation support
        .filter(|&(queue_family_index, queue_family_properties)| {
            let different_queue_family = queue_family_index as u32 != render_family;
            let transfer_support = queue_family_properties
                .queue_flags
                .contains(vk::QueueFlags::TRANSFER);
            different_queue_family && transfer_support
        })
        // some drivers expose a queue that only supports transfer operations (for this very purpose) which is preferable
        .max_by_key(|(_, q)| {
            if q.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                0
            } else {
                1
            }
        })
        .map(|(i, _)| i as u32);

    Some(ChoosePhysicalDeviceReturn {
        physical_device,
        render_queue_family_index: render_family,
        transfer_queue_family_index: transfer_family.unwrap_or(render_family),
    })
}

pub struct CreateDeviceAndQueuesReturn {
    pub device: Arc<Device>,
    pub render_queue: Arc<Queue>,
    pub transfer_queue: Arc<Queue>,
}

pub fn create_device_and_queue(
    physical_device: Arc<PhysicalDevice>,
    debug_callback: Option<Arc<DebugCallback>>,
    render_queue_family_index: u32,
    transfer_queue_family_index: u32,
) -> anyhow::Result<CreateDeviceAndQueuesReturn> {
    let separate_queue_families = render_queue_family_index != transfer_queue_family_index;

    let single_queue_priority = vec![1.0];

    let queue_infos = if separate_queue_families {
        let render_queue_priorities = &single_queue_priority;
        let render_queue_info = vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(render_queue_family_index)
            .queue_priorities(render_queue_priorities);

        let transfer_queue_priorities = &single_queue_priority;
        let transfer_queue_info = vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(transfer_queue_family_index)
            .queue_priorities(transfer_queue_priorities);

        vec![render_queue_info.build(), transfer_queue_info.build()]
    } else {
        let render_and_transfer_queue_info = vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(render_queue_family_index)
            .queue_priorities(&single_queue_priority);

        vec![render_and_transfer_queue_info.build()]
    };

    let features_1_0 = vk::PhysicalDeviceFeatures::default();
    let features_1_1 = vk::PhysicalDeviceVulkan11Features::default();
    let features_1_2 = required_features_1_2();
    let features_1_3 = vk::PhysicalDeviceVulkan13Features::default();

    let extension_names: Vec<String> = required_device_extensions()
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    // enable synchronization2 feature for vulkan api <= 1.2 (1.3 )
    let synchronization_feature =
        vk::PhysicalDeviceSynchronization2Features::builder().synchronization2(true);

    let device_raw = unsafe {
        Device::new_with_p_next_chain(
            physical_device,
            queue_infos.as_slice(),
            features_1_0,
            features_1_1,
            features_1_2,
            features_1_3,
            extension_names,
            [],
            debug_callback,
            vec![synchronization_feature],
        )?
    };
    let device = Arc::new(device_raw);

    let render_queue = Arc::new(
        Queue::new(device.clone(), render_queue_family_index, 0)
            .context("creating render queue")?,
    );
    debug!(
        "created render queue. family index = {}",
        render_queue_family_index
    );

    let transfer_queue = if separate_queue_families {
        debug!(
            "created transfer queue. family index = {}",
            transfer_queue_family_index
        );
        Arc::new(
            Queue::new(device.clone(), transfer_queue_family_index, 0)
                .context("creating transfer queue")?,
        )
    } else {
        debug!(
            "no separate transfer queue family available. transfer queue is same as render queue"
        );
        render_queue.clone()
    };

    Ok(CreateDeviceAndQueuesReturn {
        device,
        render_queue,
        transfer_queue,
    })
}

pub fn create_command_pool(device: Arc<Device>, queue: &Queue) -> anyhow::Result<Arc<CommandPool>> {
    let command_pool_props = CommandPoolProperties {
        flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
        queue_family_index: queue.family_index(),
    };
    let command_pool = CommandPool::new(device, command_pool_props)
        .context("creating render manager command pool")?;
    Ok(Arc::new(command_pool))
}

pub fn swapchain_properties(
    device: &Device,
    surface: &Surface,
    window: &Window,
) -> anyhow::Result<SwapchainProperties> {
    let preferred_image_count = MAX_FRAMES_IN_FLIGHT as u32;
    let window_dimensions: [u32; 2] = window.inner_size().into();

    let surface_capabilities = surface
        .get_physical_device_surface_capabilities(device.physical_device())
        .context("get_physical_device_surface_capabilities")?;

    let composite_alpha = choose_composite_alpha(surface_capabilities);

    let surface_formats = surface
        .get_physical_device_surface_formats(device.physical_device())
        .context("get_physical_device_surface_formats")?;
    // best practice to go with first supplied surface format
    let surface_format = surface_formats[0];

    let image_usage = vk::ImageUsageFlags::COLOR_ATTACHMENT;

    SwapchainProperties::new_default(
        device,
        surface,
        preferred_image_count,
        surface_format,
        composite_alpha,
        image_usage,
        window_dimensions,
    )
    .context("determining swapchain properties")
}

pub fn create_swapchain(
    device: Arc<Device>,
    surface: Arc<Surface>,
    window: &Window,
) -> anyhow::Result<Arc<Swapchain>> {
    let swapchain_properties = swapchain_properties(&device, &surface, window)?;
    debug!(
        "creating swapchain with dimensions: {:?}",
        swapchain_properties.width_height
    );

    let swapchain =
        Swapchain::new(device, surface, swapchain_properties).context("creating swapchain")?;
    Ok(Arc::new(swapchain))
}

pub fn create_swapchain_image_views(
    swapchain: &Swapchain,
) -> anyhow::Result<Vec<Arc<ImageView<SwapchainImage>>>> {
    let image_view_properties = swapchain.image_view_properties();

    let swapchain_images = swapchain
        .swapchain_images()
        .iter()
        .map(|image| ImageView::new(image.clone(), image_view_properties))
        .collect::<Result<Vec<_>, _>>()?;

    let swapchain_images = swapchain_images
        .into_iter()
        .map(|image_view| Arc::new(image_view))
        .collect::<Vec<_>>();

    Ok(swapchain_images)
}

/// Returns true if fragment shaders should write linear color to the swapchain image attachment.
/// Otherwise they should write srgb. Assumes color space is SRGB i.e. not HDR or something wacky like that...
///
/// See [this](https://stackoverflow.com/a/66401423/5256085) for more info on the topic.
pub fn shaders_should_write_linear_color(surface_format: vk::SurfaceFormatKHR) -> bool {
    is_format_srgb(surface_format.format)
}

/// We want a SFLOAT format for our reverse z buffer (prefer VK_FORMAT_D32_SFLOAT)
pub fn choose_depth_buffer_format(physical_device: &PhysicalDevice) -> anyhow::Result<vk::Format> {
    let d32_props = unsafe {
        physical_device
            .instance()
            .inner()
            .get_physical_device_format_properties(physical_device.handle(), vk::Format::D32_SFLOAT)
    };

    if d32_props
        .optimal_tiling_features
        .contains(vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT)
    {
        return Ok(vk::Format::D32_SFLOAT);
    }

    let d32_s8_props = unsafe {
        physical_device
            .instance()
            .inner()
            .get_physical_device_format_properties(
                physical_device.handle(),
                vk::Format::D32_SFLOAT_S8_UINT,
            )
    };

    if d32_s8_props
        .optimal_tiling_features
        .contains(vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT)
    {
        return Ok(vk::Format::D32_SFLOAT_S8_UINT);
    }

    anyhow::bail!("no sfloat depth buffer formats supported by this physical device")
}

pub mod render_pass_indices {
    pub const ATTACHMENT_SWAPCHAIN: usize = 0;
    pub const ATTACHMENT_NORMAL: usize = 1;
    pub const ATTACHMENT_PRIMITIVE_ID: usize = 2;
    pub const ATTACHMENT_DEPTH_BUFFER: usize = 3;
    pub const NUM_ATTACHMENTS: usize = 4;

    pub const SUBPASS_GBUFFER: usize = 0;
    pub const SUBPASS_DEFERRED: usize = 1;
    pub const NUM_SUBPASSES: usize = 2;
}

fn attachment_descriptions(
    swapchain_properties: &SwapchainProperties,
    depth_buffer_format: vk::Format,
) -> [vk::AttachmentDescription; render_pass_indices::NUM_ATTACHMENTS] {
    let mut attachment_descriptions =
        [vk::AttachmentDescription::default(); render_pass_indices::NUM_ATTACHMENTS];

    // swapchain
    attachment_descriptions[render_pass_indices::ATTACHMENT_SWAPCHAIN] =
        vk::AttachmentDescription::builder()
            .format(swapchain_properties.surface_format.format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .build();

    // normal buffer
    attachment_descriptions[render_pass_indices::ATTACHMENT_NORMAL] =
        vk::AttachmentDescription::builder()
            .format(FORMAT_NORMAL_BUFFER)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED) // what it will be in at the beginning of the render pass
            .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL) // what it will transition to at the end of the render pass
            .build();

    // primitive id buffer
    attachment_descriptions[render_pass_indices::ATTACHMENT_PRIMITIVE_ID] =
        vk::AttachmentDescription::builder()
            .format(FORMAT_PRIMITIVE_ID_BUFFER)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .initial_layout(vk::ImageLayout::UNDEFINED) // what it will be in at the beginning of the render pass
            .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL) // what it will transition to at the end of the render pass
            .build();

    // depth buffer
    attachment_descriptions[render_pass_indices::ATTACHMENT_DEPTH_BUFFER] =
        vk::AttachmentDescription::builder()
            .format(depth_buffer_format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED) // what it will be in at the beginning of the render pass
            .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL) // what it will transition to at the end of the render pass
            .build();

    attachment_descriptions
}

fn subpasses() -> [Subpass; render_pass_indices::NUM_SUBPASSES] {
    let mut subpasses: [Subpass; render_pass_indices::NUM_SUBPASSES] =
        [Subpass::default(), Subpass::default()];

    // g-buffer subpass

    let g_buffer_color_attachments = [
        vk::AttachmentReference::builder()
            .attachment(render_pass_indices::ATTACHMENT_NORMAL as u32)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build(),
        vk::AttachmentReference::builder()
            .attachment(render_pass_indices::ATTACHMENT_PRIMITIVE_ID as u32)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build(),
    ];

    let g_buffer_depth_attachment = vk::AttachmentReference::builder()
        .attachment(render_pass_indices::ATTACHMENT_DEPTH_BUFFER as u32)
        .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
        .build();

    subpasses[render_pass_indices::SUBPASS_GBUFFER] = Subpass::new(
        &g_buffer_color_attachments,
        Some(g_buffer_depth_attachment),
        &[],
    );

    // deferred subpass

    let deferred_color_attachments = [vk::AttachmentReference::builder()
        .attachment(render_pass_indices::ATTACHMENT_SWAPCHAIN as u32)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .build()];

    let deferred_input_attachments = [
        vk::AttachmentReference::builder()
            .attachment(render_pass_indices::ATTACHMENT_NORMAL as u32)
            .layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .build(),
        vk::AttachmentReference::builder()
            .attachment(render_pass_indices::ATTACHMENT_PRIMITIVE_ID as u32)
            .layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .build(),
    ];

    subpasses[render_pass_indices::SUBPASS_DEFERRED] = Subpass::new(
        &deferred_color_attachments,
        None,
        &deferred_input_attachments,
    );

    subpasses
}

fn subpass_dependencies() -> [vk::SubpassDependency; 2] {
    let mut subpass_dependencies = [vk::SubpassDependency::default(); 2];

    // more efficient swapchain synchronization than the implicit transition.
    // see first section of https://community.arm.com/arm-community-blogs/b/graphics-gaming-and-vr-blog/posts/vulkan-best-practices-frequently-asked-questions-part-1
    subpass_dependencies[0] = vk::SubpassDependency::builder()
        .src_subpass(vk::SUBPASS_EXTERNAL)
        .dst_subpass(render_pass_indices::SUBPASS_DEFERRED as u32)
        .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .src_access_mask(vk::AccessFlags::empty())
        .dst_access_mask(
            vk::AccessFlags::COLOR_ATTACHMENT_WRITE | vk::AccessFlags::COLOR_ATTACHMENT_READ,
        )
        .build();

    // input attachments
    subpass_dependencies[1] = vk::SubpassDependency::builder()
        .src_subpass(render_pass_indices::SUBPASS_GBUFFER as u32)
        .dst_subpass(render_pass_indices::SUBPASS_DEFERRED as u32)
        .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .dst_stage_mask(vk::PipelineStageFlags::FRAGMENT_SHADER)
        .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
        .dst_access_mask(vk::AccessFlags::SHADER_READ)
        .build();

    subpass_dependencies
}

pub fn create_render_pass(
    device: Arc<Device>,
    swapchain_properties: &SwapchainProperties,
    depth_buffer_format: vk::Format,
) -> anyhow::Result<Arc<RenderPass>> {
    let attachment_descriptions =
        attachment_descriptions(swapchain_properties, depth_buffer_format);
    let subpasses = subpasses();
    let subpass_dependencies = subpass_dependencies();

    let render_pass = RenderPass::new(
        device,
        attachment_descriptions,
        subpasses,
        subpass_dependencies,
    )
    .context("creating render pass")?;
    Ok(Arc::new(render_pass))
}

pub fn create_depth_buffer(
    memory_allocator: Arc<MemoryAllocator>,
    dimensions: ImageDimensions,
    depth_buffer_format: vk::Format,
) -> anyhow::Result<Arc<ImageView<Image>>> {
    let image = Image::new_tranient(
        memory_allocator,
        dimensions,
        depth_buffer_format,
        vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
    )
    .context("creating depth buffer image")?;

    let image_view_properties =
        ImageViewProperties::from_image_properties_default(image.properties());
    let image_view = ImageView::new(Arc::new(image), image_view_properties)
        .context("creating depth buffer image view")?;
    Ok(Arc::new(image_view))
}

pub fn create_normal_buffer(
    memory_allocator: Arc<MemoryAllocator>,
    dimensions: ImageDimensions,
) -> anyhow::Result<Arc<ImageView<Image>>> {
    let image = Image::new_tranient(
        memory_allocator,
        dimensions,
        FORMAT_NORMAL_BUFFER,
        vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::INPUT_ATTACHMENT,
    )
    .context("creating normal buffer image")?;

    let image_view_properties =
        ImageViewProperties::from_image_properties_default(image.properties());
    let image_view = ImageView::new(Arc::new(image), image_view_properties)
        .context("creating normal buffer image view")?;
    Ok(Arc::new(image_view))
}

/// Creates `framebuffer_count` number of primitive id buffer image views
pub fn create_primitive_id_buffers(
    framebuffer_count: usize,
    memory_allocator: Arc<MemoryAllocator>,
    dimensions: ImageDimensions,
) -> anyhow::Result<Vec<Arc<ImageView<Image>>>> {
    (0..framebuffer_count)
        .into_iter()
        .map(|_| create_primitive_id_buffer(memory_allocator.clone(), dimensions))
        .collect::<anyhow::Result<Vec<_>>>()
}

fn create_primitive_id_buffer(
    memory_allocator: Arc<MemoryAllocator>,
    dimensions: ImageDimensions,
) -> anyhow::Result<Arc<ImageView<Image>>> {
    let image_properties = ImageProperties::new_default(
        FORMAT_PRIMITIVE_ID_BUFFER,
        dimensions,
        vk::ImageUsageFlags::COLOR_ATTACHMENT
            | vk::ImageUsageFlags::INPUT_ATTACHMENT
            | vk::ImageUsageFlags::TRANSFER_SRC,
    );

    let allocation_info = AllocationCreateInfo {
        required_flags: vk::MemoryPropertyFlags::DEVICE_LOCAL,
        ..AllocationCreateInfo::default()
    };

    let image = Image::new(memory_allocator, image_properties, allocation_info)
        .context("creating primitive id buffer image")?;

    let image_view_properties =
        ImageViewProperties::from_image_properties_default(image.properties());
    let image_view = ImageView::new(Arc::new(image), image_view_properties)
        .context("creating primitive id buffer image view")?;
    Ok(Arc::new(image_view))
}

pub fn create_cpu_read_staging_buffer(
    memory_allocator: Arc<MemoryAllocator>,
) -> anyhow::Result<Buffer> {
    let buffer_properties =
        BufferProperties::new_default(CPU_ACCESS_BUFFER_SIZE, vk::BufferUsageFlags::TRANSFER_DST);

    // prefer host cached over device local because we'll be writing via gpu and reading from cpu [see here for more info](https://asawicki.info/news_1740_vulkan_memory_types_on_pc_and_how_to_use_them)
    let allocation_info = AllocationCreateInfo {
        required_flags: vk::MemoryPropertyFlags::HOST_VISIBLE,
        preferred_flags: vk::MemoryPropertyFlags::HOST_COHERENT
            | vk::MemoryPropertyFlags::HOST_CACHED,
        ..AllocationCreateInfo::default()
    };

    let buffer = Buffer::new(memory_allocator, buffer_properties, allocation_info)?;
    Ok(buffer)
}

/// Safety:
/// * `primitive_id_buffers` must contain `framebuffer_count` elements.
/// * if `swapchain_image_views` contains more than one image, it must contain
///   `framebuffer_count` elements.
pub fn create_framebuffers(
    render_pass: &Arc<RenderPass>,
    swapchain_image_views: &Vec<Arc<ImageView<SwapchainImage>>>,
    normal_buffer: &Arc<ImageView<Image>>,
    primitive_id_buffers: &Vec<Arc<ImageView<Image>>>,
    depth_buffer: &Arc<ImageView<Image>>,
) -> anyhow::Result<Vec<Arc<Framebuffer>>> {
    (0..swapchain_image_views.len())
        .into_iter()
        .map(|i| {
            let mut attachments = Vec::<Arc<dyn ImageViewAccess>>::with_capacity(
                render_pass_indices::NUM_ATTACHMENTS,
            );
            attachments.insert(
                render_pass_indices::ATTACHMENT_SWAPCHAIN,
                swapchain_image_views[i].clone(),
            );
            attachments.insert(
                render_pass_indices::ATTACHMENT_NORMAL,
                normal_buffer.clone(),
            );
            attachments.insert(
                render_pass_indices::ATTACHMENT_PRIMITIVE_ID,
                primitive_id_buffers[i].clone(),
            );
            attachments.insert(
                render_pass_indices::ATTACHMENT_DEPTH_BUFFER,
                depth_buffer.clone(),
            );

            let framebuffer_properties = FramebufferProperties::new_default(
                attachments,
                swapchain_image_views[i].image().dimensions(),
            );
            let framebuffer = Framebuffer::new(render_pass.clone(), framebuffer_properties)
                .context("creating framebuffer")?;
            Ok(Arc::new(framebuffer))
        })
        .collect::<anyhow::Result<Vec<_>>>()
}

pub fn create_clear_values() -> Vec<vk::ClearValue> {
    let mut clear_values =
        Vec::<vk::ClearValue>::with_capacity(render_pass_indices::NUM_ATTACHMENTS);
    clear_values.insert(
        render_pass_indices::ATTACHMENT_SWAPCHAIN,
        vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0., 0., 0., 1.],
            },
        },
    );
    clear_values.insert(
        render_pass_indices::ATTACHMENT_NORMAL,
        vk::ClearValue {
            color: vk::ClearColorValue { float32: [0.; 4] },
        },
    );
    clear_values.insert(
        render_pass_indices::ATTACHMENT_PRIMITIVE_ID,
        vk::ClearValue {
            color: vk::ClearColorValue {
                uint32: [PRIMITIVE_ID_INVALID; 4],
            },
        },
    );
    clear_values.insert(
        render_pass_indices::ATTACHMENT_DEPTH_BUFFER,
        vk::ClearValue {
            depth_stencil: vk::ClearDepthStencilValue {
                depth: 0.,
                stencil: 0,
            },
        },
    );
    clear_values
}

pub fn create_camera_ubo(memory_allocator: Arc<MemoryAllocator>) -> anyhow::Result<Arc<Buffer>> {
    let ubo_size = mem::size_of::<CameraUniformBuffer>() as vk::DeviceSize;
    let ubo_props = BufferProperties::new_default(ubo_size, vk::BufferUsageFlags::UNIFORM_BUFFER);

    let alloc_info = allocation_info_cpu_accessible();
    let buffer = Buffer::new(memory_allocator, ubo_props, alloc_info)
        .context("creating camera ubo buffer")?;
    Ok(Arc::new(buffer))
}

pub fn create_render_command_buffers(
    render_command_pool: Arc<CommandPool>,
    swapchain_image_count: u32,
) -> anyhow::Result<Vec<Arc<CommandBuffer>>> {
    let command_buffers = render_command_pool
        .allocate_command_buffers(vk::CommandBufferLevel::PRIMARY, swapchain_image_count)
        .context("allocating per-frame command buffers")?;

    let command_buffer_arcs = command_buffers
        .into_iter()
        .map(|cb| Arc::new(cb))
        .collect::<Vec<_>>();
    Ok(command_buffer_arcs)
}

pub fn create_camera_descriptor_set_with_binding(
    descriptor_pool: Arc<DescriptorPool>,
    binding: u32,
) -> VkResult<Arc<DescriptorSet>> {
    let desc_set_layout_props =
        DescriptorSetLayoutProperties::new_default(vec![DescriptorSetLayoutBinding {
            binding,
            descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: 1,
            stage_flags: vk::ShaderStageFlags::FRAGMENT | vk::ShaderStageFlags::VERTEX,
            ..Default::default()
        }]);

    let desc_set_layout = Arc::new(DescriptorSetLayout::new(
        descriptor_pool.device().clone(),
        desc_set_layout_props,
    )?);

    let desc_set = descriptor_pool.allocate_descriptor_set(desc_set_layout)?;

    Ok(Arc::new(desc_set))
}

pub fn write_camera_descriptor_set(
    desc_set_camera: &DescriptorSet,
    camera_buffer: &Buffer,
    binding: u32,
) {
    let camera_buffer_info = vk::DescriptorBufferInfo {
        buffer: camera_buffer.handle(),
        offset: 0,
        range: mem::size_of::<CameraUniformBuffer>() as vk::DeviceSize,
    };
    let camera_buffer_infos = [camera_buffer_info];

    let descriptor_write = vk::WriteDescriptorSet::builder()
        .dst_set(desc_set_camera.handle())
        .dst_binding(binding)
        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
        .buffer_info(&camera_buffer_infos);

    desc_set_camera
        .device()
        .update_descriptor_sets([descriptor_write], []);
}

pub fn create_shader_stages_from_bytes(
    device: &Arc<Device>,
    mut vertex_spv_file: std::io::Cursor<&[u8]>,
    mut frag_spv_file: std::io::Cursor<&[u8]>,
) -> Result<(ShaderStage, ShaderStage), ShaderError> {
    let vert_shader = Arc::new(ShaderModule::new_from_spirv(
        device.clone(),
        &mut vertex_spv_file,
    )?);
    let frag_shader = Arc::new(ShaderModule::new_from_spirv(
        device.clone(),
        &mut frag_spv_file,
    )?);

    Ok(create_shader_stages_from_modules(vert_shader, frag_shader))
}

pub fn create_shader_stages_from_path(
    device: &Arc<Device>,
    vert_shader_file_path: &str,
    frag_shader_file_path: &str,
) -> Result<(ShaderStage, ShaderStage), ShaderError> {
    let vert_shader = Arc::new(ShaderModule::new_from_file(
        device.clone(),
        vert_shader_file_path,
    )?);
    let frag_shader = Arc::new(ShaderModule::new_from_file(
        device.clone(),
        frag_shader_file_path,
    )?);

    Ok(create_shader_stages_from_modules(vert_shader, frag_shader))
}

pub fn create_shader_stages_from_modules(
    vert_shader: Arc<ShaderModule>,
    frag_shader: Arc<ShaderModule>,
) -> (ShaderStage, ShaderStage) {
    let vert_stage = ShaderStage::new(
        vk::ShaderStageFlags::VERTEX,
        vert_shader,
        CString::new(SHADER_ENTRY_POINT).expect("SHADER_ENTRY_POINT shouldn't contain null byte"),
        None,
    );
    let frag_stage = ShaderStage::new(
        vk::ShaderStageFlags::FRAGMENT,
        frag_shader,
        CString::new(SHADER_ENTRY_POINT).expect("SHADER_ENTRY_POINT shouldn't contain null byte"),
        None,
    );

    (vert_stage, frag_stage)
}
