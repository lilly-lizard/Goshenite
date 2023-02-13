use crate::{
    device::Device,
    image_base::ImageBase,
    image_properties::{
        default_component_mapping, default_subresource_range, ImageDimensions, ImageViewProperties,
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
    dimensions: ImageDimensions,

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
        let component_mapping = default_component_mapping();

        let layer_count = swapchain.properties().array_layers;
        let view_type = if layer_count > 1 {
            vk::ImageViewType::TYPE_2D_ARRAY
        } else {
            vk::ImageViewType::TYPE_2D
        };
        let subresource_range = vk::ImageSubresourceRange {
            layer_count,
            ..default_subresource_range(vk::ImageAspectFlags::COLOR)
        };

        let image_view_properties = ImageViewProperties {
            format,
            view_type,
            component_mapping,
            subresource_range,
            ..ImageViewProperties::default()
        };

        let image_view_create_info_builder =
            image_view_properties.create_info_builder(image_handle);
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
            dimensions: swapchain.properties().dimensions(),

            device,
            _swapchain: swapchain,
        })
    }

    #[inline]
    pub fn layer_count(&self) -> u32 {
        self.image_view_properties.subresource_range.layer_count
    }
}

impl ImageBase for SwapchainImage {
    fn image_handle(&self) -> vk::Image {
        self.image_handle
    }

    fn image_view_handle(&self) -> vk::ImageView {
        self.image_view_handle
    }

    fn dimensions(&self) -> ImageDimensions {
        self.dimensions
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
