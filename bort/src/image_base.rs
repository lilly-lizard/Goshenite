use ash::vk;

pub trait ImageBase {
    fn image_handle(&self) -> vk::Image;
    fn image_view_handle(&self) -> vk::ImageView;
    fn extent(&self) -> vk::Extent3D;
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

pub fn extent_2d_from_dimensions(dimensions: [u32; 2]) -> vk::Extent2D {
    vk::Extent2D {
        width: dimensions[0],
        height: dimensions[1],
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

pub fn whole_viewport(image: &dyn ImageBase) -> vk::Viewport {
    vk::Viewport {
        x: 0.,
        y: 0.,
        width: image.extent().width as f32,
        height: image.extent().height as f32,
        min_depth: 0.,
        max_depth: image.extent().depth as f32,
    }
}

// Image Properties

/// WARNING `default()` values for `format`, `extent` and `usage` are nothing!
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
    pub sharing_mode: vk::SharingMode,
    pub queue_family_indices: Vec<u32>,
    pub initial_layout: vk::ImageLayout,
    pub image_create_flags: vk::ImageCreateFlags,
}

impl Default for ImageProperties {
    fn default() -> Self {
        Self {
            image_type: vk::ImageType::TYPE_2D,
            mip_levels: 1,
            array_layers: 1,
            samples: vk::SampleCountFlags::TYPE_1,
            tiling: vk::ImageTiling::OPTIMAL,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_indices: Vec::new(),
            initial_layout: vk::ImageLayout::GENERAL,
            image_create_flags: vk::ImageCreateFlags::empty(),

            // nonsense defaults. make sure you override these!
            format: vk::Format::default(),
            extent: vk::Extent3D::default(),
            usage: vk::ImageUsageFlags::empty(),
        }
    }
}

impl ImageProperties {
    pub fn new_default(
        format: vk::Format,
        dimensions: [u32; 2],
        usage: vk::ImageUsageFlags,
        initial_layout: vk::ImageLayout,
    ) -> Self {
        Self {
            format,
            extent: extent_3d_from_dimensions(dimensions),
            usage,
            initial_layout,
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
            .sharing_mode(self.sharing_mode)
            .initial_layout(self.initial_layout);
        if self.queue_family_indices.len() > 0 {
            builder = builder.queue_family_indices(self.queue_family_indices.as_slice());
        }

        builder
    }
}

#[derive(Debug, Copy, Clone)]
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
    pub image_view_properties: ImageViewProperties,
    pub extent: vk::Extent3D,
}

impl ImageBase for ImageRaw {
    fn image_handle(&self) -> vk::Image {
        self.image_handle
    }

    fn image_view_handle(&self) -> vk::ImageView {
        self.image_view_handle
    }

    fn extent(&self) -> vk::Extent3D {
        self.extent
    }

    fn image_view_properties(&self) -> ImageViewProperties {
        self.image_view_properties
    }
}
