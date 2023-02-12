use crate::{device::Device, swapchain::Swapchain, ALLOCATION_CALLBACK};
use anyhow::Context;
use ash::vk;
use std::sync::Arc;

pub fn default_component_mapping() -> vk::ComponentMapping {
    vk::ComponentMapping {
        r: vk::ComponentSwizzle::R,
        g: vk::ComponentSwizzle::G,
        b: vk::ComponentSwizzle::B,
        a: vk::ComponentSwizzle::A,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ImageProperties {
    pub format: vk::Format,
    pub dimensions: [u32; 2],
    pub view_type: vk::ImageViewType,
    pub component_mapping: vk::ComponentMapping,
    pub subresource_range: vk::ImageSubresourceRange,
}

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
        let subresource_range = vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
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
                dimensions,
                view_type,
                component_mapping,
                subresource_range,
            },

            device,
            _swapchain: swapchain,
        })
    }

    pub fn viewport(&self) -> vk::Viewport {
        vk::Viewport {
            x: 0.,
            y: 0.,
            width: self.properties.dimensions[0] as f32,
            height: self.properties.dimensions[1] as f32,
            min_depth: 0.,
            max_depth: 1.,
        }
    }

    // Getters

    pub fn image_handle(&self) -> vk::Image {
        self.image_handle
    }

    pub fn image_view_handle(&self) -> vk::ImageView {
        self.image_view_handle
    }

    pub fn properties(&self) -> ImageProperties {
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
