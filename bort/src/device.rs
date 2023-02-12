use crate::{
    common::string_to_c_string_vec,
    instance::{ApiVersion, Instance},
    physical_device::PhysicalDevice,
    ALLOCATION_CALLBACK,
};
use anyhow::Context;
use ash::vk;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

pub struct Device {
    pub inner: ash::Device,
}

impl Device {
    /// `features_1_1` and `features_1_2` may be ignored depending on the `instance` api version.
    pub fn new<'a>(
        instance: &Instance,
        physical_device: &PhysicalDevice,
        queue_create_infos: &'a [vk::DeviceQueueCreateInfo],
        features_1_0: vk::PhysicalDeviceFeatures,
        mut features_1_1: vk::PhysicalDeviceVulkan11Features,
        mut features_1_2: vk::PhysicalDeviceVulkan12Features,
        extension_names: impl IntoIterator<Item = String>,
        layer_names: impl IntoIterator<Item = String>,
    ) -> anyhow::Result<Self> {
        let extension_names_raw = string_to_c_string_vec(extension_names)
            .context("converting extension names to c strings")?;
        let layer_names_raw =
            string_to_c_string_vec(layer_names).context("converting layer names to c strings")?;

        let mut device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(queue_create_infos)
            .enabled_extension_names(extension_names_raw.as_slice())
            .enabled_layer_names(layer_names_raw.as_slice());

        let mut features_2 = vk::PhysicalDeviceFeatures2::builder();
        if instance.api_version() <= ApiVersion::new(1, 0) {
            device_create_info = device_create_info.enabled_features(&features_1_0);
        } else {
            features_2 = features_2.features(features_1_0);
            device_create_info = device_create_info.push_next(&mut features_2);

            if instance.api_version() >= ApiVersion::new(1, 1) {
                device_create_info = device_create_info.push_next(&mut features_1_1)
            }

            if instance.api_version() >= ApiVersion::new(1, 2) {
                device_create_info = device_create_info.push_next(&mut features_1_2);
            }
        }

        let inner = unsafe {
            instance
                .inner()
                .create_device(physical_device.handle(), &device_create_info, None)
        }
        .context("creating vulkan device")?;

        Ok(Self { inner })
    }

    pub fn wait_idle(&self) -> anyhow::Result<()> {
        unsafe { self.inner.device_wait_idle().context("vkDeviceWaitIdle") }
    }

    pub fn inner(&self) -> &ash::Device {
        &self.inner
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        self.wait_idle().expect("vkDeviceWaitIdle");
        unsafe {
            self.inner.destroy_device(ALLOCATION_CALLBACK);
        }
    }
}
