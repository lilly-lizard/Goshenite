use ash::vk;

pub trait ImageBase {
    fn image_handle(&self) -> vk::Image;
    fn image_view_handle(&self) -> vk::ImageView;
    fn properties(&self) -> ImageProperties;
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

#[derive(Debug, Clone)]
pub struct ImageProperties {
    // image
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

    // view
    pub view_type: vk::ImageViewType,
    pub component_mapping: vk::ComponentMapping,
    pub subresource_range: vk::ImageSubresourceRange,
}

impl ImageProperties {
    pub fn image_create_info(&self) -> vk::ImageCreateInfo {
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

        builder.build()
    }

    pub fn sharing_mode(&self) -> vk::SharingMode {
        if self.queue_family_indices.is_some() {
            vk::SharingMode::CONCURRENT
        } else {
            vk::SharingMode::EXCLUSIVE
        }
    }
}

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
