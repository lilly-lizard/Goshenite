use crate::{
    device::Device,
    image::Image,
    image_base::{
        default_component_mapping, default_subresource_range, ImageBase, ImageProperties,
    },
    swapchain::Swapchain,
    ALLOCATION_CALLBACK,
};
use anyhow::Context;
use ash::vk;
use std::sync::Arc;

pub struct SwapchainImage {
    image_handle: vk::Image,
    image_view_handle: vk::ImageView,
    properties: ImageProperties,

    // dependencies
    device: Arc<Device>,
    _swapchain: Arc<Swapchain>,
}

impl SwapchainImage {
    pub fn from_swapchain(
        device: Arc<Device>,
        swapchain: Arc<Swapchain>,
    ) -> anyhow::Result<Vec<Self>> {
        swapchain
            .get_swapchain_images()
            .context("getting swapchain images")?
            .into_iter()
            .map(|image_handle| {
                Self::from_image_handle(
                    device.clone(),
                    swapchain.clone(),
                    image_handle,
                    swapchain.properties().surface_format.format,
                    swapchain.properties().dimensions,
                )
            })
            .collect::<anyhow::Result<Vec<_>>>()
    }

    fn from_image_handle(
        device: Arc<Device>,
        swapchain: Arc<Swapchain>,
        image_handle: vk::Image,
        format: vk::Format,
        dimensions: [u32; 2],
    ) -> anyhow::Result<Self> {
        let view_type = vk::ImageViewType::TYPE_2D;
        let component_mapping = default_component_mapping();
        let subresource_range = default_subresource_range(vk::ImageAspectFlags::COLOR);
        let extent = vk::Extent3D {
            width: dimensions[0],
            height: dimensions[1],
            depth: 1,
        };

        let image_view_info = vk::ImageViewCreateInfo::builder()
            .view_type(view_type)
            .format(format)
            .components(component_mapping)
            .subresource_range(subresource_range)
            .image(image_handle)
            .build();

        let image_view_handle = unsafe {
            device
                .inner()
                .create_image_view(&image_view_info, ALLOCATION_CALLBACK)
        }
        .context("create_image_view")?;

        Ok(Self {
            image_handle,
            image_view_handle,
            properties: ImageProperties {
                format,
                extent,
                view_type,
                component_mapping,
                subresource_range,
            },

            device,
            _swapchain: swapchain,
        })
    }
}

impl ImageBase for SwapchainImage {
    fn image_handle(&self) -> vk::Image {
        self.image_handle
    }

    fn image_view_handle(&self) -> vk::ImageView {
        self.image_view_handle
    }

    fn properties(&self) -> ImageProperties {
        self.properties
    }
}

impl Drop for SwapchainImage {
    fn drop(&mut self) {
        // note we shouldn't destroy the swapchain images. that'll be handled by the `Swapchain`.
        unsafe {
            self.device
                .inner()
                .destroy_image_view(self.image_view_handle, ALLOCATION_CALLBACK);
        }
    }
}
