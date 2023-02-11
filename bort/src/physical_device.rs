use crate::{
    instance::{ApiVersion, Instance},
    surface::Surface,
};
use anyhow::Context;
use ash::{
    prelude::VkResult,
    vk::{self, api_version_major, api_version_minor},
    Entry,
};
use std::{ffi::CStr, str::Utf8Error};

/// Properties of an extension in the loader or a physical device.
#[derive(Clone, Debug)]
pub struct ExtensionProperties {
    pub extension_name: String,
    pub spec_version: u32,
}

impl ExtensionProperties {
    fn new(value: vk::ExtensionProperties) -> Result<Self, Utf8Error> {
        let c_str = unsafe { CStr::from_ptr(value.extension_name.as_ptr()) };
        let extension_name = c_str.to_str()?.to_string();
        Ok(Self {
            extension_name,
            spec_version: value.spec_version,
        })
    }
}

pub struct PhysicalDevice {
    handle: vk::PhysicalDevice,
    properties: vk::PhysicalDeviceProperties,
    features: vk::PhysicalDeviceFeatures,
    queue_family_properties: Vec<vk::QueueFamilyProperties>,
    memory_properties: vk::PhysicalDeviceMemoryProperties,
    extension_properties: Vec<ExtensionProperties>,
}

impl PhysicalDevice {
    pub fn new(instance: &Instance, handle: vk::PhysicalDevice) -> anyhow::Result<Self> {
        let features = unsafe { instance.inner().get_physical_device_features(handle) };
        let properties = unsafe { instance.inner().get_physical_device_properties(handle) };

        let queue_family_properties = unsafe {
            instance
                .inner()
                .get_physical_device_queue_family_properties(handle)
        };
        let memory_properties = unsafe {
            instance
                .inner()
                .get_physical_device_memory_properties(handle)
        };

        let vk_extension_properties = unsafe {
            instance
                .inner()
                .enumerate_device_extension_properties(handle)
        }?;
        let extension_properties: Vec<ExtensionProperties> = vk_extension_properties
            .into_iter()
            .map(|props| {
                ExtensionProperties::new(props)
                    .context("processing physical device extension properties")
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            handle,
            properties,
            features,
            queue_family_properties,
            memory_properties,
            extension_properties,
        })
    }

    pub fn supports_api_ver(&self, api_version: ApiVersion) -> bool {
        let supported_major = api_version_major(self.properties.api_version);
        let supported_minor = api_version_minor(self.properties.api_version);
        if supported_major < api_version.major {
            return false;
        }
        if supported_minor < api_version.minor {
            return false;
        }
        return true;
    }

    pub fn supports_extensions<'a>(
        &self,
        mut extension_names: impl Iterator<Item = &'a String>,
    ) -> bool {
        extension_names.all(|extension_name| {
            self.extension_properties
                .iter()
                .any(|props| props.extension_name == *extension_name)
        })
    }

    pub fn supports_extension(&self, extension_name: String) -> bool {
        self.extension_properties
            .iter()
            .any(|props| props.extension_name == extension_name)
    }

    // Getters

    pub fn handle(&self) -> vk::PhysicalDevice {
        self.handle
    }

    pub fn properties(&self) -> vk::PhysicalDeviceProperties {
        self.properties
    }

    pub fn features(&self) -> vk::PhysicalDeviceFeatures {
        self.features
    }

    pub fn queue_family_properties(&self) -> &Vec<vk::QueueFamilyProperties> {
        &self.queue_family_properties
    }

    pub fn memory_properties(&self) -> vk::PhysicalDeviceMemoryProperties {
        self.memory_properties
    }

    pub fn extension_properties(&self) -> &Vec<ExtensionProperties> {
        &self.extension_properties
    }
}
