use crate::{common::string_to_c_string_vec, memory::ALLOCATION_CALLBACK_NONE};
use anyhow::Context;
use ash::{extensions::ext::DebugUtils, vk, Entry};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use raw_window_handle::RawDisplayHandle;
use std::ffi::CString;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ApiVersion {
    pub major: u32,
    pub minor: u32,
}

impl ApiVersion {
    pub fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }
}

#[test]
fn api_version_ordering() {
    let ver_1_1 = ApiVersion::new(1, 1);
    let ver_1_2 = ApiVersion::new(1, 2);
    assert!(ver_1_1 < ver_1_2);
}

pub struct Instance {
    inner: ash::Instance,
    api_version: ApiVersion,
}

impl Instance {
    /// No need to specify display extensions or debug validation layer/extension, this function will figure that out for you.
    pub fn new(
        entry: &Entry,
        api_version: ApiVersion,
        app_name: &str,
        display_handle: RawDisplayHandle,
        enable_debug_validation: bool,
        additional_layer_names: impl IntoIterator<Item = String>,
        additional_extension_names: impl IntoIterator<Item = String>,
    ) -> anyhow::Result<Self> {
        let app_name = CString::new(app_name).context("converting app name to c string")?;
        let appinfo = vk::ApplicationInfo::builder()
            .application_name(&app_name)
            .application_version(0)
            .engine_name(&app_name)
            .engine_version(0)
            .api_version(vk::make_api_version(
                0,
                api_version.major,
                api_version.minor,
                0,
            ));

        let mut layer_names_raw = string_to_c_string_vec(additional_layer_names)
            .context("converting layer names to c strings")?;
        let mut extension_names_raw = string_to_c_string_vec(additional_extension_names)
            .context("converting extension names to c strings")?;

        let display_extension_names = ash_window::enumerate_required_extensions(display_handle)
            .context("querying required display extensions")?;
        extension_names_raw.extend_from_slice(display_extension_names);

        let validation_layer_name =
            CString::new("VK_LAYER_KHRONOS_validation").expect("no nulls in str");
        if enable_debug_validation {
            layer_names_raw.push(validation_layer_name.as_ptr());
        }
        if enable_debug_validation {
            extension_names_raw.push(DebugUtils::name().as_ptr());
        }

        trace!("enabling instance extensions: {:?}", extension_names_raw);
        trace!("enabling vulkan layers: {:?}", layer_names_raw);

        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&appinfo)
            .enabled_layer_names(&layer_names_raw)
            .enabled_extension_names(&extension_names_raw);

        let instance = unsafe {
            entry
                .create_instance(&create_info, ALLOCATION_CALLBACK_NONE)
                .context("creating vulkan instance")?
        };

        Ok(Self {
            inner: instance,
            api_version,
        })
    }

    /// Vulkan 1.0 features
    pub fn physical_device_features_1_0(
        &self,
        physical_device_handle: vk::PhysicalDevice,
    ) -> vk::PhysicalDeviceFeatures {
        unsafe {
            self.inner()
                .get_physical_device_features(physical_device_handle)
        }
    }

    /// Vulkan 1.1 features. If api version < 1.1, these cannot be populated.
    pub fn physical_device_features_1_1(
        &self,
        physical_device_handle: vk::PhysicalDevice,
    ) -> Option<vk::PhysicalDeviceVulkan11Features> {
        if self.api_version < ApiVersion::new(1, 1) {
            return None;
        }

        let mut features_1_1 = vk::PhysicalDeviceVulkan11Features::default();
        let mut features = vk::PhysicalDeviceFeatures2::builder()
            .push_next(&mut features_1_1)
            .build();
        unsafe {
            self.inner
                .get_physical_device_features2(physical_device_handle, &mut features)
        };

        Some(features_1_1)
    }

    /// Vulkan 1.2 features. If api version < 1.2, these cannot be populated.
    pub fn physical_device_features_1_2(
        &self,
        physical_device_handle: vk::PhysicalDevice,
    ) -> Option<vk::PhysicalDeviceVulkan12Features> {
        if self.api_version < ApiVersion::new(1, 2) {
            return None;
        }

        let mut features_1_2 = vk::PhysicalDeviceVulkan12Features::default();
        let mut features = vk::PhysicalDeviceFeatures2::builder()
            .push_next(&mut features_1_2)
            .build();
        unsafe {
            self.inner
                .get_physical_device_features2(physical_device_handle, &mut features)
        };

        Some(features_1_2)
    }

    // Getters

    pub fn inner(&self) -> &ash::Instance {
        &self.inner
    }

    pub fn api_version(&self) -> ApiVersion {
        self.api_version
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            self.inner.destroy_instance(ALLOCATION_CALLBACK_NONE);
        }
    }
}
