use super::config_renderer::{FRAMES_IN_FLIGHT, VULKAN_VER_MAJ, VULKAN_VER_MIN};
use anyhow::Context;
use ash::vk;
use bort::{
    common::is_format_srgb, device::Device, instance::Instance, physical_device::PhysicalDevice,
    queue::Queue, surface::Surface, swapchain::Swapchain,
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
        .filter_map(|p| check_physical_device_queue_support(p, surface, instance))
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
    instance: &Instance,
    physical_device: &PhysicalDevice,
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
        instance,
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
    instance: &Instance,
    device: &Device,
    surface: &Surface,
    window: &Window,
    physical_device: &PhysicalDevice,
) -> anyhow::Result<Swapchain> {
    let preferred_image_count = FRAMES_IN_FLIGHT as u32;
    let window_dimensions: [u32; 2] = window.inner_size().into();

    let surface_formats = surface
        .get_physical_device_surface_formats(physical_device)
        .context("get_physical_device_surface_formats")?;
    let preferred_surface_format = surface_formats
        .iter()
        .cloned()
        // use the first SRGB format we find
        .find(|vk::SurfaceFormatKHR { format, .. }| is_format_srgb(*format))
        // otherwise just go with the first format
        .unwrap_or(surface_formats[0]);

    Swapchain::new(
        instance,
        device,
        surface,
        physical_device,
        preferred_surface_format,
        preferred_image_count,
        window_dimensions,
    )
}
