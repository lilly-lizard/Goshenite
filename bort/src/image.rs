use crate::{
    device::Device,
    image_base::{ImageBase, ImageProperties, ImageViewProperties},
    ALLOCATION_CALLBACK,
};
use anyhow::Context;
use ash::vk;
use std::sync::Arc;

pub struct Image {
    image_handle: vk::Image,
    image_properties: ImageProperties,
    image_view_handle: vk::ImageView,
    image_view_properties: ImageViewProperties,

    // dependencies
    device: Arc<Device>,
}

impl Image {
    pub fn new(
        device: Arc<Device>,
        image_properties: ImageProperties,
        image_view_properties: ImageViewProperties,
    ) -> anyhow::Result<Self> {
        let image_handle = unsafe {
            device
                .inner()
                .create_image(&image_properties.create_info_builder(), ALLOCATION_CALLBACK)
        }
        .context("creating image")?;

        let image_view_handle = unsafe {
            device.inner().create_image_view(
                &image_view_properties.create_info_builder(image_handle),
                ALLOCATION_CALLBACK,
            )
        }
        .context("creating image view")?;

        Ok(Self {
            image_handle,
            image_properties,
            image_view_handle,
            image_view_properties,
            device,
        })
    }
}

impl ImageBase for Image {
    fn image_handle(&self) -> vk::Image {
        self.image_handle
    }

    fn image_view_handle(&self) -> vk::ImageView {
        self.image_view_handle
    }

    fn extent(&self) -> vk::Extent3D {
        self.image_properties.extent
    }

    fn image_view_properties(&self) -> ImageViewProperties {
        self.image_view_properties
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        unsafe {
            self.device
                .inner()
                .destroy_image_view(self.image_view_handle, ALLOCATION_CALLBACK);
            self.device
                .inner()
                .destroy_image(self.image_handle, ALLOCATION_CALLBACK);
        }
    }
}
