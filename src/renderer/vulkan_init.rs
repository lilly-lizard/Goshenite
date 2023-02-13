use super::config_renderer::{
    FORMAT_DEPTH_BUFFER, FORMAT_G_BUFFER_NORMAL, FORMAT_G_BUFFER_PRIMITIVE_ID, FRAMES_IN_FLIGHT,
    VULKAN_VER_MAJ, VULKAN_VER_MIN,
};
use anyhow::Context;
use ash::vk;
use bort::{
    device::Device,
    instance::Instance,
    physical_device::PhysicalDevice,
    queue::Queue,
    render_pass::{RenderPass, Subpass},
    surface::Surface,
    swapchain::{choose_composite_alpha, get_first_srgb_surface_format, Swapchain},
};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::sync::Arc;
use winit::window::Window;

pub fn required_device_extensions() -> [&'static str; 2] {
    ["VK_KHR_swapchain", "VK_EXT_descriptor_indexing"]
}

/// Make sure to update `required_features_1_2` too!
pub fn supports_required_features_1_2(
    supported_features: vk::PhysicalDeviceVulkan12Features,
) -> bool {
    supported_features.descriptor_indexing == vk::TRUE
        && supported_features.runtime_descriptor_array == vk::TRUE
        && supported_features.descriptor_binding_variable_descriptor_count == vk::TRUE
        && supported_features.shader_storage_buffer_array_non_uniform_indexing == vk::TRUE
        && supported_features.descriptor_binding_partially_bound == vk::TRUE
}
/// Make sure to update `supports_required_features_1_2` too!
pub fn required_features_1_2() -> vk::PhysicalDeviceVulkan12Features {
    vk::PhysicalDeviceVulkan12Features {
        descriptor_indexing: vk::TRUE,
        runtime_descriptor_array: vk::TRUE,
        descriptor_binding_variable_descriptor_count: vk::TRUE,
        shader_storage_buffer_array_non_uniform_indexing: vk::TRUE,
        descriptor_binding_partially_bound: vk::TRUE,
        ..vk::PhysicalDeviceVulkan12Features::default()
    }
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
        .filter(|p| p.supports_api_ver(instance.api_version()))
        // filter for required device extensionssupports_extension
        .filter(|p| p.supports_extensions(required_extensions.into_iter()))
        // filter for queue support
        .filter_map(|p| check_physical_device_queue_support(p, surface, &instance))
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
        .position(|(i, q)| {
            // must support our surface and essential operations
            q.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                && q.queue_flags.contains(vk::QueueFlags::TRANSFER)
                && surface
                    .get_physical_device_surface_support(&physical_device, i as u32)
                    .unwrap_or(false)
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
    if supports_required_features_1_2(supported_features) {
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
        .filter(|(i, q)| {
            *i as u32 != render_family && q.queue_flags.contains(vk::QueueFlags::TRANSFER)
        })
        // some drivers expose a queue that only supports transfer operations (for this very purpose) which is preferable
        .max_by_key(|(_, q)| {
            if !q.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                1
            } else {
                0
            }
        })
        .map(|(i, _)| i as u32);

    Some(ChoosePhysicalDeviceReturn {
        physical_device: physical_device,
        render_queue_family_index: render_family,
        transfer_queue_family_index: transfer_family.unwrap_or(render_family),
    })
}

pub struct CreateDeviceAndQueuesReturn {
    pub device: Arc<Device>,
    pub render_queue: Queue,
    pub transfer_queue: Option<Queue>,
}

pub fn create_device_and_queues(
    physical_device: Arc<PhysicalDevice>,
    render_queue_family_index: u32,
    transfer_queue_family_index: u32,
) -> anyhow::Result<CreateDeviceAndQueuesReturn> {
    let queue_priorities = [1.0];
    let single_queue = transfer_queue_family_index != render_queue_family_index;

    let render_queue_info = vk::DeviceQueueCreateInfo::builder()
        .queue_family_index(render_queue_family_index)
        .queue_priorities(&queue_priorities);
    let mut queue_infos = vec![render_queue_info.build()];

    let mut transfer_queue_info = vk::DeviceQueueCreateInfo::builder();
    if single_queue {
        transfer_queue_info = transfer_queue_info
            .queue_family_index(transfer_queue_family_index)
            .queue_priorities(&queue_priorities);
        queue_infos.push(transfer_queue_info.build());
    }

    let features_1_0 = vk::PhysicalDeviceFeatures::default();
    let features_1_1 = vk::PhysicalDeviceVulkan11Features::default();
    let features_1_2 = required_features_1_2();

    let extension_names: Vec<String> = required_device_extensions()
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    let device = Arc::new(Device::new(
        physical_device,
        queue_infos.as_slice(),
        features_1_0,
        features_1_1,
        features_1_2,
        extension_names,
        [],
    )?);

    let render_queue = Queue::new(device.clone(), render_queue_family_index, 0);

    let transfer_queue = if single_queue {
        None
    } else {
        Some(Queue::new(device.clone(), transfer_queue_family_index, 0))
    };

    Ok(CreateDeviceAndQueuesReturn {
        device,
        render_queue,
        transfer_queue,
    })
}

pub fn create_swapchain(
    device: Arc<Device>,
    surface: Arc<Surface>,
    window: &Window,
    physical_device: &PhysicalDevice,
) -> anyhow::Result<Swapchain> {
    let preferred_image_count = FRAMES_IN_FLIGHT as u32;
    let window_dimensions: [u32; 2] = window.inner_size().into();

    let surface_capabilities = surface
        .get_physical_device_surface_capabilities(physical_device)
        .context("get_physical_device_surface_capabilities")?;

    let composite_alpha = choose_composite_alpha(surface_capabilities);

    let surface_formats = surface
        .get_physical_device_surface_formats(physical_device)
        .context("get_physical_device_surface_formats")?;
    let surface_format = get_first_srgb_surface_format(&surface_formats);

    let image_usage = vk::ImageUsageFlags::COLOR_ATTACHMENT;

    Swapchain::new(
        device,
        surface,
        preferred_image_count,
        surface_format,
        composite_alpha,
        image_usage,
        window_dimensions,
    )
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
    swapchain: &Swapchain,
) -> [vk::AttachmentDescription; render_pass_indices::NUM_ATTACHMENTS] {
    let mut attachment_descriptions =
        [vk::AttachmentDescription::default(); render_pass_indices::NUM_ATTACHMENTS];

    // swapchain
    attachment_descriptions[render_pass_indices::ATTACHMENT_SWAPCHAIN] =
        vk::AttachmentDescription::builder()
            .format(swapchain.properties().surface_format.format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .build();

    // normal buffer
    attachment_descriptions[render_pass_indices::ATTACHMENT_NORMAL] =
        vk::AttachmentDescription::builder()
            .format(FORMAT_G_BUFFER_NORMAL)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED) // what it will be in at the beginning of the render pass
            .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL) // what it will transition to at the end of the render pass
            .build();

    // primitive id buffer
    attachment_descriptions[render_pass_indices::ATTACHMENT_PRIMITIVE_ID] =
        vk::AttachmentDescription::builder()
            .format(FORMAT_G_BUFFER_PRIMITIVE_ID)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED) // what it will be in at the beginning of the render pass
            .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL) // what it will transition to at the end of the render pass
            .build();

    // depth buffer
    attachment_descriptions[render_pass_indices::ATTACHMENT_DEPTH_BUFFER] =
        vk::AttachmentDescription::builder()
            .format(FORMAT_DEPTH_BUFFER)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED) // what it will be in at the beginning of the render pass
            .final_layout(vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL) // what it will transition to at the end of the render pass
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
            .attachment(render_pass_indices::ATTACHMENT_NORMAL as u32)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build(),
    ];

    let g_buffer_depth_attachment = vk::AttachmentReference::builder()
        .attachment(render_pass_indices::ATTACHMENT_DEPTH_BUFFER as u32)
        .layout(vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL)
        .build();

    subpasses[render_pass_indices::SUBPASS_GBUFFER] =
        Subpass::new(g_buffer_color_attachments, g_buffer_depth_attachment, []);

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
        deferred_color_attachments,
        Default::default(),
        deferred_input_attachments,
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
        .src_stage_mask(vk::PipelineStageFlags::FRAGMENT_SHADER)
        .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
        .dst_access_mask(vk::AccessFlags::SHADER_READ)
        .build();

    subpass_dependencies
}

pub fn create_render_pass(
    device: Arc<Device>,
    swapchain: &Swapchain,
) -> anyhow::Result<RenderPass> {
    let attachment_descriptions = attachment_descriptions(swapchain);
    let subpasses = subpasses();
    let subpass_dependencies = subpass_dependencies();

    RenderPass::new(
        device,
        attachment_descriptions,
        subpasses,
        subpass_dependencies,
    )
    .context("creating render pass")
}
