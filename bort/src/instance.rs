use crate::{common::to_c_string_vec, ALLOCATION_CALLBACK};
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
        additional_layer_names: Vec<String>,
        additional_extension_names: Vec<String>,
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

        let mut layer_names_raw = to_c_string_vec(additional_layer_names)
            .context("converting layer names to c strings")?;
        let mut extension_names_raw = to_c_string_vec(additional_extension_names)
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

        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&appinfo)
            .enabled_layer_names(&layer_names_raw)
            .enabled_extension_names(&extension_names_raw);

        let instance = unsafe {
            entry
                .create_instance(&create_info, ALLOCATION_CALLBACK)
                .context("creating vulkan instance")?
        };

        Ok(Self {
            inner: instance,
            api_version,
        })
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
            self.inner.destroy_instance(ALLOCATION_CALLBACK);
        }
    }
}
