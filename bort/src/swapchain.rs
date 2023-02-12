use anyhow::Context;
use ash::{extensions::khr, vk};
use std::cmp::{max, min};

use crate::{
    device::Device, instance::Instance, physical_device::PhysicalDevice, surface::Surface,
    ALLOCATION_CALLBACK,
};

pub struct Swapchain {
    handle: vk::SwapchainKHR,
    swapchain_loader: khr::Swapchain,
    surface_format: vk::SurfaceFormatKHR,
    image_count: u32,
    dimensions: vk::Extent2D,
    present_mode: vk::PresentModeKHR,
    composite_alpha: vk::CompositeAlphaFlagsKHR,
}

impl Swapchain {
    /// Prefers the following settings:
    /// - present mode = `vk::PresentModeKHR::MAILBOX`
    /// - pre-transform = `vk::SurfaceTransformFlagsKHR::IDENTITY`
    ///
    /// If preferred parameters aren't supported, defaults to the following:
    /// - image count clamped based on `vk::SurfaceCapabilitiesKHR`
    ///
    /// `surface_format`, `composite_alpha` and `image_usage` and are unchecked.
    ///
    /// Sharing mode is set to `vk::SharingMode::EXCLUSIVE` and clipping is enabled.
    pub fn new(
        instance: &Instance,
        device: &Device,
        surface: &Surface,
        physical_device: &PhysicalDevice,
        preferred_image_count: u32,
        surface_format: vk::SurfaceFormatKHR,
        composite_alpha: vk::CompositeAlphaFlagsKHR,
        image_usage: vk::ImageUsageFlags,
        window_dimensions: [u32; 2],
    ) -> anyhow::Result<Self> {
        let swapchain_loader = khr::Swapchain::new(instance.inner(), device.inner());

        let surface_capabilities = surface
            .get_physical_device_surface_capabilities(physical_device)
            .context("get_physical_device_surface_capabilities")?;

        let image_count = max(
            min(preferred_image_count, surface_capabilities.max_image_count),
            surface_capabilities.min_image_count,
        );

        let dimensions = match surface_capabilities.current_extent.width {
            std::u32::MAX => vk::Extent2D {
                width: window_dimensions[0],
                height: window_dimensions[1],
            },
            _ => surface_capabilities.current_extent,
        };

        let present_modes = surface
            .get_physical_device_surface_present_modes(physical_device)
            .context("get_physical_device_surface_present_modes")?;
        let present_mode = present_modes
            .into_iter()
            .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
            .unwrap_or(vk::PresentModeKHR::FIFO);

        // should think more about this if targeting mobile in the future...
        let pre_transform = if surface_capabilities
            .supported_transforms
            .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
        {
            vk::SurfaceTransformFlagsKHR::IDENTITY
        } else {
            surface_capabilities.current_transform
        };

        let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(surface.handle())
            .min_image_count(image_count)
            .image_color_space(surface_format.color_space)
            .image_format(surface_format.format)
            .image_extent(dimensions)
            .image_usage(image_usage)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(pre_transform)
            .composite_alpha(composite_alpha)
            .present_mode(present_mode)
            .clipped(true)
            .image_array_layers(1);

        let handle = unsafe {
            swapchain_loader.create_swapchain(&swapchain_create_info, ALLOCATION_CALLBACK)
        }
        .context("creating swapchain")?;

        Ok(Self {
            handle,
            swapchain_loader,
            surface_format,
            image_count,
            dimensions,
            present_mode,
            composite_alpha,
        })
    }

    // Getters

    pub fn handle(&self) -> vk::SwapchainKHR {
        self.handle
    }

    pub fn swapchain_loader(&self) -> &khr::Swapchain {
        &self.swapchain_loader
    }

    pub fn surface_format(&self) -> vk::SurfaceFormatKHR {
        self.surface_format
    }

    pub fn image_count(&self) -> u32 {
        self.image_count
    }

    pub fn dimensions(&self) -> vk::Extent2D {
        self.dimensions
    }

    pub fn present_mode(&self) -> vk::PresentModeKHR {
        self.present_mode
    }

    pub fn composite_alpha(&self) -> vk::CompositeAlphaFlagsKHR {
        self.composite_alpha
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            self.swapchain_loader
                .destroy_swapchain(self.handle, ALLOCATION_CALLBACK)
        };
    }
}
