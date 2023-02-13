use crate::{
    device::Device,
    ALLOCATION_CALLBACK, image_base::{ImageProperties, ImageBase},
};
use ash::vk;
use std::sync::Arc;

pub struct Image {
    image_handle: vk::Image,
    image_view_handle: vk::ImageView,
    properties: ImageProperties,

    // dependencies
    device: Arc<Device>,
}

impl Image {
    pub fn new_default(
        device: Arc<Device>,

        format: vk::Format,
        dimensions: [u32; 2],
        usage: vk::ImageUsageFlags,
        initial_layout: vk::ImageLayout,
        image_aspect_mask: vk::ImageAspectFlags,
    ) -> Self {
        Self::new(
            device,
            format,
            dimensions,
            1,
            1,
            vk::SampleCountFlags::TYPE_1,
            vk::ImageTiling::OPTIMAL,
            usage,
            None,
            initial_layout,
            image_aspect_mask,
        )
    }

    pub fn new(
        device: Arc<Device>,
		properties: 
    ) -> Self {
        let image_type = vk::ImageType::TYPE_2D;
        let extent = vk::Extent3D {
            width: dimensions[0],
            height: dimensions[1],
            depth: 1,
        };
        let sharing_mode = if queue_family_indices.is_some() {
            vk::SharingMode::CONCURRENT
        } else {
            vk::SharingMode::EXCLUSIVE
        };

        let image_handle = unsafe {
            device
                .inner()
                .create_image(create_info, ALLOCATION_CALLBACK)
        };

        let view_type = if array_layers > 1 {
            vk::ImageViewType::TYPE_2D_ARRAY
        } else {
            vk::ImageViewType::TYPE_2D
        };
        let component_mapping = default_component_mapping();
        let subresource_range = default_subresource_range(image_aspect_mask);

        Self {
            image_handle,
            image_view_handle,
            properties: ImageProperties {
                // image
                image_type,
                format,
                extent,
                mip_levels,
                array_layers,
                samples,
                tiling,
                usage,
                sharing_mode,
                queue_family_indices,
                initial_layout,

                // view
                view_type,
                component_mapping,
                subresource_range,
            },
            device,
        }
    }
}

impl ImageBase for Image {
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
