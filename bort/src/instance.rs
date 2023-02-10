use crate::ALLOCATION_CALLBACK;
use anyhow::Context;
use ash::{vk, vk::InstanceCreateInfo, Entry};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::ffi::{CStr, CString};

pub struct Instance {
    pub inner: ash::Instance,
}

impl Instance {
    pub fn new(
        entry: &Entry,
        ver_major: u32,
        ver_minor: u32,
        app_name: &str,
    ) -> anyhow::Result<Self> {
        let app_name = CString::new(app_name).context("converting app name to c string")?;
        let appinfo = vk::ApplicationInfo::builder()
            .application_name(&app_name)
            .application_version(0)
            .engine_name(&app_name)
            .engine_version(0)
            .api_version(vk::make_api_version(0, 1, 0, 0));

        let create_info = InstanceCreateInfo::builder()
            .application_info(&appinfo)
            .enabled_layer_names(&layers_names_raw)
            .enabled_extension_names(&extension_names)
            .flags(create_flags);

        let instance = unsafe {
            entry
                .create_instance(&create_info, ALLOCATION_CALLBACK)
                .context("creating vulkan instance")?
        };

        Ok(Self { inner: instance })
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            self.inner.destroy_instance(ALLOCATION_CALLBACK);
        }
    }
}
