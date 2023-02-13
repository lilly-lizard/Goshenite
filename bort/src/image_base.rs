use ash::vk;

pub trait ImageBase {
    fn image_handle(&self) -> vk::Image;
    fn image_view_handle(&self) -> vk::ImageView;
    fn extent(&self) -> vk::Extent3D;
    fn image_view_properties(&self) -> ImageViewProperties;
}

// Helper funtions

pub fn extent_from_dimensions(dimensions: [u32; 2]) -> vk::Extent3D {
    vk::Extent3D {
        width: dimensions[0],
        height: dimensions[1],
        depth: 1,
    }
}

pub fn default_component_mapping() -> vk::ComponentMapping {
    vk::ComponentMapping {
        r: vk::ComponentSwizzle::R,
        g: vk::ComponentSwizzle::G,
        b: vk::ComponentSwizzle::B,
        a: vk::ComponentSwizzle::A,
    }
}

pub fn default_subresource_range(aspect_mask: vk::ImageAspectFlags) -> vk::ImageSubresourceRange {
    vk::ImageSubresourceRange {
        aspect_mask,
        base_mip_level: 0,
        level_count: 1,
        base_array_layer: 0,
        layer_count: 1,
    }
}

fn whole_viewport(image: &dyn ImageBase) -> vk::Viewport {
    vk::Viewport {
        x: 0.,
        y: 0.,
        width: image.properties().extent.width as f32,
        height: image.properties().extent.height as f32,
        min_depth: 0.,
        max_depth: image.properties().extent.depth as f32,
    }
}

// Image Properties

/// Default values are for a 2D srgb rgba32 color image dimensions = [1, 1, 1] and no usage flags.
#[derive(Debug, Clone)]
pub struct ImageProperties {
    pub image_type: vk::ImageType,
    pub format: vk::Format,
    pub extent: vk::Extent3D,
    pub mip_levels: u32,
    pub array_layers: u32,
    pub samples: vk::SampleCountFlags,
    pub tiling: vk::ImageTiling,
    pub usage: vk::ImageUsageFlags,
    pub queue_family_indices: Option<Vec<u32>>,
    pub initial_layout: vk::ImageLayout,
    pub image_create_flags: vk::ImageCreateFlags,
}

impl Default for ImageProperties {
    fn default() -> Self {
        Self {
            image_type: vk::ImageType::TYPE_2D,
            format: vk::Format::R8G8B8A8_SRGB,
            extent: extent_from_dimensions([1, 1]),
            mip_levels: 1,
            array_layers: 1,
            samples: vk::SampleCountFlags::TYPE_1,
            tiling: vk::ImageTiling::OPTIMAL,
            usage: vk::ImageUsageFlags::empty(),
            queue_family_indices: None,
            initial_layout: vk::ImageLayout::GENERAL,
            image_create_flags: vk::ImageCreateFlags::empty(),
        }
    }
}

impl ImageProperties {
    pub fn new_default(
        format: vk::Format,
        dimensions: [u32; 2],
        usage: vk::ImageUsageFlags,
        initial_layout: vk::ImageLayout,
        image_aspect_mask: vk::ImageAspectFlags,
    ) -> Self {
        Self {
            format: vk::Format,
            extent: extent_from_dimensions(dimensions),
            usage: vk::ImageUsageFlags,
            initial_layout: vk::ImageLayout,
            ..Self::default()
        }
    }

    pub fn create_info_builder(&self) -> vk::ImageCreateInfoBuilder {
        let mut builder = vk::ImageCreateInfo::builder()
            .flags(self.image_create_flags)
            .image_type(self.image_type)
            .format(self.format)
            .extent(self.extent)
            .mip_levels(self.mip_levels)
            .array_layers(self.array_layers)
            .samples(self.samples)
            .tiling(self.tiling)
            .usage(self.usage)
            .sharing_mode(self.sharing_mode())
            .initial_layout(self.initial_layout);
        if let Some(queue_family_indices_ref) = self.queue_family_indices {
            builder = builder.queue_family_indices(queue_family_indices_ref.as_slice());
        }

        builder
    }

    pub fn sharing_mode(&self) -> vk::SharingMode {
        if self.queue_family_indices.is_some() {
            vk::SharingMode::CONCURRENT
        } else {
            vk::SharingMode::EXCLUSIVE
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImageViewProperties {
    pub view_type: vk::ImageViewType,
    pub component_mapping: vk::ComponentMapping,
    pub format: vk::Format,
    pub subresource_range: vk::ImageSubresourceRange,
    pub image_view_create_flags: vk::ImageViewCreateFlags,
}

impl Default for ImageViewProperties {
    fn default() -> Self {
        Self {
            view_type: vk::ImageViewType::TYPE_2D,
            component_mapping: default_component_mapping(),
            format: vk::Format::R8G8B8A8_SRGB,
            subresource_range: default_subresource_range(vk::ImageAspectFlags::COLOR),
            image_view_create_flags: vk::ImageViewCreateFlags::empty(),
        }
    }
}

impl ImageViewProperties {
    pub fn new_default(format: vk::Format, image_aspect_mask: vk::ImageAspectFlags) -> Self {
        Self {
            format,
            subresource_range: default_subresource_range(image_aspect_mask),
            ..Self::default()
        }
    }

    pub fn create_info_builder(&self, image_handle: vk::Image) -> vk::ImageViewCreateInfoBuilder {
        vk::ImageViewCreateInfo::builder()
            .flags(self.image_view_create_flags)
            .image(image_handle)
            .view_type(self.view_type)
            .format(self.format)
            .components(self.component_mapping)
            .subresource_range(self.subresource_range)
    }
}

// Image Raw

pub struct ImageRaw {
    pub image_handle: vk::Image,
    pub image_view_handle: vk::ImageView,
    pub properties: ImageProperties,
}

impl ImageBase for ImageRaw {
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
