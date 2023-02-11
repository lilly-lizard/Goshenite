use std::sync::Arc;

use crate::{instance::Instance, physical_device::PhysicalDevice, ALLOCATION_CALLBACK};
use ash::{extensions::khr, prelude::VkResult, vk::SurfaceKHR, Entry};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

pub struct Surface {
    inner: SurfaceKHR,
    surface_loader: khr::Surface,

    // dependencies
    _instance: Arc<Instance>,
}

impl Surface {
    pub fn new(
        entry: &Entry,
        instance: Arc<Instance>,
        raw_display_handle: RawDisplayHandle,
        raw_window_handle: RawWindowHandle,
    ) -> VkResult<Self> {
        let inner = unsafe {
            ash_window::create_surface(
                entry,
                instance.inner(),
                raw_display_handle,
                raw_window_handle,
                ALLOCATION_CALLBACK,
            )
        }?;

        let surface_loader = khr::Surface::new(&entry, instance.inner());

        Ok(Self {
            inner,
            surface_loader,

            _instance: instance,
        })
    }

    pub fn get_physical_device_surface_support(
        &self,
        physical_device: &PhysicalDevice,
        queue_family_index: u32,
    ) -> VkResult<bool> {
        unsafe {
            self.surface_loader.get_physical_device_surface_support(
                physical_device.handle(),
                queue_family_index,
                self.inner,
            )
        }
    }

    // Getters

    pub fn inner(&self) -> &SurfaceKHR {
        &self.inner
    }

    pub fn surface_loader(&self) -> &khr::Surface {
        &self.surface_loader
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.surface_loader
                .destroy_surface(self.inner, ALLOCATION_CALLBACK)
        };
    }
}
