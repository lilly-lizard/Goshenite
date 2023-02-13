use ash::vk;

// Image Properties

/// WARNING `default()` values for `format`, `dimensions` and `usage` are nothing!
#[derive(Debug, Clone)]
pub struct ImageProperties {
    pub format: vk::Format,
    pub dimensions: ImageDimensions,
    pub mip_levels: u32,
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
            mip_levels: 1,
            samples: vk::SampleCountFlags::TYPE_1,
            tiling: vk::ImageTiling::OPTIMAL,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_indices: Vec::new(),
            initial_layout: vk::ImageLayout::GENERAL,
            image_create_flags: vk::ImageCreateFlags::empty(),

            // nonsense defaults. make sure you override these!
            format: vk::Format::default(),
            dimensions: ImageDimensions::default(),
            usage: vk::ImageUsageFlags::empty(),
        }
    }
}

impl ImageProperties {
    pub fn new_default(
        format: vk::Format,
        dimensions: ImageDimensions,
        usage: vk::ImageUsageFlags,
        initial_layout: vk::ImageLayout,
    ) -> Self {
        Self {
            format,
            dimensions,
            usage,
            initial_layout,
            ..Self::default()
        }
    }

    pub fn create_info_builder(&self) -> vk::ImageCreateInfoBuilder {
        let mut builder = vk::ImageCreateInfo::builder()
            .flags(self.image_create_flags)
            .image_type(self.dimensions.image_type())
            .format(self.format)
            .extent(self.dimensions.extent_3d())
            .mip_levels(self.mip_levels)
            .array_layers(self.dimensions.array_layers())
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

// Image View Properties

/// WARNING `default()` values for `format`, `view_type` are nothing!
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
            component_mapping: default_component_mapping(),
            image_view_create_flags: vk::ImageViewCreateFlags::empty(),
            subresource_range: default_subresource_range(vk::ImageAspectFlags::COLOR),

            // nonsense defaults. make sure you override these!
            format: vk::Format::default(),
            view_type: vk::ImageViewType::default(),
        }
    }
}

impl ImageViewProperties {
    pub fn from_image_properties(image_properties: ImageProperties) -> Self {
        let format = image_properties.format;
        let view_type = image_properties.dimensions.default_image_view_type();
        let aspect_mask = aspect_mask_from_format(format);
        let subresource_range = vk::ImageSubresourceRange {
            aspect_mask,
            base_mip_level: 0,
            level_count: image_properties.mip_levels,
            base_array_layer: 0,
            layer_count: image_properties.dimensions.array_layers(),
        };

        Self {
            format,
            subresource_range,
            view_type,
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

// Image Dimensions

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ImageDimensions {
    Dim1d {
        width: u32,
        array_layers: u32,
    },
    Dim2d {
        width: u32,
        height: u32,
        array_layers: u32,
    },
    Dim3d {
        width: u32,
        height: u32,
        depth: u32,
    },
}

impl ImageDimensions {
    pub fn width(&self) -> u32 {
        match *self {
            ImageDimensions::Dim1d { width, .. } => width,
            ImageDimensions::Dim2d { width, .. } => width,
            ImageDimensions::Dim3d { width, .. } => width,
        }
    }

    pub fn height(&self) -> u32 {
        match *self {
            ImageDimensions::Dim1d { .. } => 1,
            ImageDimensions::Dim2d { height, .. } => height,
            ImageDimensions::Dim3d { height, .. } => height,
        }
    }

    pub fn depth(&self) -> u32 {
        match *self {
            ImageDimensions::Dim1d { .. } => 1,
            ImageDimensions::Dim2d { .. } => 1,
            ImageDimensions::Dim3d { depth, .. } => depth,
        }
    }

    pub fn extent_3d(&self) -> vk::Extent3D {
        vk::Extent3D {
            width: self.width(),
            height: self.height(),
            depth: self.depth(),
        }
    }

    pub fn array_layers(&self) -> u32 {
        match *self {
            ImageDimensions::Dim1d { array_layers, .. } => array_layers,
            ImageDimensions::Dim2d { array_layers, .. } => array_layers,
            ImageDimensions::Dim3d { .. } => 1,
        }
    }

    /// Returns the total number of texels for an image of these dimensions.
    pub fn num_texels(&self) -> u32 {
        self.width() * self.height() * self.depth() * self.array_layers()
    }

    pub fn image_type(&self) -> vk::ImageType {
        match *self {
            ImageDimensions::Dim1d { .. } => vk::ImageType::TYPE_1D,
            ImageDimensions::Dim2d { .. } => vk::ImageType::TYPE_2D,
            ImageDimensions::Dim3d { .. } => vk::ImageType::TYPE_3D,
        }
    }

    pub fn default_image_view_type(&self) -> vk::ImageViewType {
        match self {
            Self::Dim1d {
                array_layers: 1, ..
            } => vk::ImageViewType::TYPE_1D,
            Self::Dim1d { .. } => vk::ImageViewType::TYPE_1D_ARRAY,
            Self::Dim2d {
                array_layers: 1, ..
            } => vk::ImageViewType::TYPE_2D,
            Self::Dim2d { .. } => vk::ImageViewType::TYPE_2D_ARRAY,
            Self::Dim3d { .. } => vk::ImageViewType::TYPE_3D,
        }
    }

    pub fn whole_viewport(&self) -> vk::Viewport {
        vk::Viewport {
            x: 0.,
            y: 0.,
            width: self.width() as f32,
            height: self.height() as f32,
            min_depth: 0.,
            max_depth: self.depth() as f32,
        }
    }
}

impl Default for ImageDimensions {
    fn default() -> Self {
        Self::Dim1d {
            width: 1,
            array_layers: 1,
        }
    }
}

// Helper Functions

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

/// Doesn't support planes/metadata.
pub fn aspect_mask_from_format(format: vk::Format) -> vk::ImageAspectFlags {
    let mut aspect = vk::ImageAspectFlags::empty();

    if !matches!(
        format,
        vk::Format::D16_UNORM
            | vk::Format::X8_D24_UNORM_PACK32
            | vk::Format::D32_SFLOAT
            | vk::Format::S8_UINT
            | vk::Format::D16_UNORM_S8_UINT
            | vk::Format::D24_UNORM_S8_UINT
            | vk::Format::D32_SFLOAT_S8_UINT
    ) {
        aspect |= vk::ImageAspectFlags::COLOR;
    }

    if matches!(
        format,
        vk::Format::D16_UNORM
            | vk::Format::X8_D24_UNORM_PACK32
            | vk::Format::D32_SFLOAT
            | vk::Format::D16_UNORM_S8_UINT
            | vk::Format::D24_UNORM_S8_UINT
            | vk::Format::D32_SFLOAT_S8_UINT
    ) {
        aspect |= vk::ImageAspectFlags::DEPTH;
    }

    if matches!(
        format,
        vk::Format::S8_UINT
            | vk::Format::D16_UNORM_S8_UINT
            | vk::Format::D24_UNORM_S8_UINT
            | vk::Format::D32_SFLOAT_S8_UINT
    ) {
        aspect |= vk::ImageAspectFlags::STENCIL;
    }

    aspect
}
