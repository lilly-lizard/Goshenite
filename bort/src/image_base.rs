use crate::image_properties::{ImageDimensions, ImageViewProperties};
use ash::vk;

pub trait ImageBase {
    fn image_handle(&self) -> vk::Image;
    fn image_view_handle(&self) -> vk::ImageView;
    fn dimensions(&self) -> ImageDimensions;
    fn image_view_properties(&self) -> ImageViewProperties;
}

// Helper funtions

pub fn extent_3d_from_dimensions(dimensions: [u32; 2]) -> vk::Extent3D {
    vk::Extent3D {
        width: dimensions[0],
        height: dimensions[1],
        depth: 1,
    }
}

pub fn extent_2d_from_width_height(dimensions: [u32; 2]) -> vk::Extent2D {
    vk::Extent2D {
        width: dimensions[0],
        height: dimensions[1],
    }
}

// Image Raw

pub struct ImageRaw {
    pub image_handle: vk::Image,
    pub image_view_handle: vk::ImageView,
    pub image_view_properties: ImageViewProperties,
    pub dimensions: ImageDimensions,
}

impl ImageBase for ImageRaw {
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
