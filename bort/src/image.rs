use crate::{
    device::Device,
    image_base::ImageBase,
    image_properties::{ImageDimensions, ImageProperties, ImageViewProperties},
    memory::{find_memorytype_index, ALLOCATION_CALLBACK_NONE},
};
use anyhow::Context;
use ash::vk;
use std::sync::Arc;

pub struct Image {
    image_handle: vk::Image,
    image_properties: ImageProperties,
    image_memory: vk::DeviceMemory,

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
            device.inner().create_image(
                &image_properties.create_info_builder(),
                ALLOCATION_CALLBACK_NONE,
            )
        }
        .context("creating image")?;

        let image_view_handle = unsafe {
            device.inner().create_image_view(
                &image_view_properties.create_info_builder(image_handle),
                ALLOCATION_CALLBACK_NONE,
            )
        }
        .context("creating image view")?;

        // MEMORY BEGIN

        let image_memory = create_and_bind_image_memory(&device, image_handle)?;

        // MEMORY END

        Ok(Self {
            image_handle,
            image_properties,
            image_memory,
            image_view_handle,
            image_view_properties,
            device,
        })
    }
}

fn create_and_bind_image_memory(
    device: &Device,
    image_handle: vk::Image,
) -> anyhow::Result<vk::DeviceMemory> {
    let device_memory_properties = unsafe {
        device
            .instance()
            .inner()
            .get_physical_device_memory_properties(device.physical_device().handle())
    };

    let image_memory_reqs = unsafe { device.inner().get_image_memory_requirements(image_handle) };

    let image_memory_index = find_memorytype_index(
        &image_memory_reqs,
        &device_memory_properties,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )
    .context("unable to find suitable memory index for image")?;

    let image_allocate_info = vk::MemoryAllocateInfo::builder()
        .allocation_size(image_memory_reqs.size)
        .memory_type_index(image_memory_index);

    let image_memory = unsafe {
        device
            .inner()
            .allocate_memory(&image_allocate_info, ALLOCATION_CALLBACK_NONE)
    }
    .context("allocating image memory")?;

    unsafe {
        device
            .inner()
            .bind_image_memory(image_handle, image_memory, 0)
    }
    .context("binding image memory")?;

    Ok(image_memory)
}

impl ImageBase for Image {
    fn image_handle(&self) -> vk::Image {
        self.image_handle
    }

    fn image_view_handle(&self) -> vk::ImageView {
        self.image_view_handle
    }

    fn dimensions(&self) -> ImageDimensions {
        self.image_properties.dimensions
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
                .free_memory(self.image_memory, ALLOCATION_CALLBACK_NONE);
            self.device
                .inner()
                .destroy_image_view(self.image_view_handle, ALLOCATION_CALLBACK_NONE);
            self.device
                .inner()
                .destroy_image(self.image_handle, ALLOCATION_CALLBACK_NONE);
        }
    }
}
