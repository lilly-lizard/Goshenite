use crate::{
    device::Device,
    image::Image,
    image_base::{
        default_component_mapping, default_subresource_range, extent_from_dimensions, ImageBase,
        ImageProperties, ImageViewProperties,
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
    image_view_properties: ImageViewProperties,
    dimensions: [u32; 2],

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
                Self::from_image_handle(device.clone(), swapchain.clone(), image_handle)
            })
            .collect::<anyhow::Result<Vec<_>>>()
    }

    fn from_image_handle(
        device: Arc<Device>,
        swapchain: Arc<Swapchain>,
        image_handle: vk::Image,
    ) -> anyhow::Result<Self> {
        let format = swapchain.properties().surface_format.format;
        let extent = extent_from_dimensions(swapchain.properties().dimensions);

        let component_mapping = default_component_mapping();
        let subresource_range = default_subresource_range(vk::ImageAspectFlags::COLOR);

        let image_view_properties = ImageViewProperties {
            format,
            view_type: vk::ImageViewType::TYPE_2D,
            component_mapping,
            subresource_range,
            ..Default::default()
        };

        let image_view_create_info_builder = image_properties.create_info_builder(image_handle);
        let image_view_handle = unsafe {
            device
                .inner()
                .create_image_view(&image_view_create_info_builder, ALLOCATION_CALLBACK)
        }
        .context("create_image_view")?;

        Ok(Self {
            image_handle,
            image_view_handle,
            image_view_properties,
            dimensions: swapchain.properties().dimensions,

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

    fn extent(&self) -> vk::Extent3D {
        extent_from_dimensions(self.dimensions)
    }

    fn image_view_properties(&self) -> ImageViewProperties {
        self.image_view_properties
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
