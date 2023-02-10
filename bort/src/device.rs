use crate::ALLOCATION_CALLBACK;
use anyhow::Context;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

pub struct Device {
    pub inner: ash::Device,
}

impl Device {
    pub fn wait_idle(&self) -> anyhow::Result<()> {
        unsafe { self.inner.device_wait_idle().context("vkDeviceWaitIdle") }
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
