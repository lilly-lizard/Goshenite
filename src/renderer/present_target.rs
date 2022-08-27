// todo commenting
// todo unwraps

use crate::immutable::Immutable;
use ash::{
    extensions::khr::{Surface, Swapchain},
    vk,
};
pub use ash::{Device, Instance};
use winit::window::Window;

pub struct PresentTarget {
    // Immutable members describing vulkan environment
    // todo weak device ptr?
    device: Immutable<Device>,
    physical_device: Immutable<vk::PhysicalDevice>,
    surface_loader: Immutable<Surface>,
    surface: Immutable<vk::SurfaceKHR>,
    swapchain_loader: Immutable<Swapchain>, // contains instance functions for swapchain extension

    // Presentation objects and info
    swapchain: vk::SwapchainKHR,
    swapchain_image_views: Vec<vk::ImageView>,
    format: vk::SurfaceFormatKHR,
    resolution: vk::Extent2D,
    image_count: u32,
}

// Getters
impl PresentTarget {
    pub fn swapchain_loader(&self) -> &Swapchain {
        &self.swapchain_loader
    }
    pub fn format(&self) -> vk::SurfaceFormatKHR {
        self.format
    }
    pub fn swapchain(&self) -> &vk::SwapchainKHR {
        &self.swapchain
    }
    pub fn swapchain_image_views(&self) -> &Vec<vk::ImageView> {
        &self.swapchain_image_views
    }
    pub fn image_count(&self) -> u32 {
        self.image_count
    }
    pub fn resolution(&self) -> vk::Extent2D {
        self.resolution
    }
}

// Public functions
impl PresentTarget {
    pub fn new(
        window: &Window,
        instance: &Instance,
        device: Device,
        surface_loader: Surface,
        physical_device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
    ) -> Self {
        let mut present_target = PresentTarget {
            swapchain_loader: Immutable::new(Swapchain::new(instance, &device)),
            device: Immutable::new(device),
            physical_device: Immutable::new(physical_device),
            surface_loader: Immutable::new(surface_loader),
            surface: Immutable::new(surface),
            /* set in create() */ swapchain: vk::SwapchainKHR::default(),
            /* set in create() */ swapchain_image_views: Vec::default(),
            /* set in create() */ format: vk::SurfaceFormatKHR::default(),
            /* set in create() */ resolution: vk::Extent2D::default(),
            /* set in create() */ image_count: u32::default(),
        };
        present_target.create(window);
        present_target
    }

    /// Recreate the swapchain due to e.g. window resize
    pub fn recreate(&mut self, window: &Window) {
        self.cleanup();
        self.create(window);
    }

    /// Destroy vulkan objects owned by this struct and reinitialize data fields
    pub fn cleanup(&mut self) {
        self.destroy_vk_objs();
        self.set_defaults();
    }
}

// Private functions
impl PresentTarget {
    /// Create swapchain/images and set render target data
    fn create(&mut self, window: &Window) {
        unsafe {
            // present mode
            let present_mode = self
                .surface_loader
                .get_physical_device_surface_present_modes(*self.physical_device, *self.surface)
                .unwrap()
                .iter()
                .cloned()
                .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
                .unwrap_or(vk::PresentModeKHR::FIFO);

            // choose format todo preference
            self.format = self
                .surface_loader
                .get_physical_device_surface_formats(*self.physical_device, *self.surface)
                .unwrap()[0];

            // use surface capabilities data
            let surface_capabilities = self
                .surface_loader
                .get_physical_device_surface_capabilities(*self.physical_device, *self.surface)
                .unwrap();

            // resolution
            self.resolution = match surface_capabilities.current_extent.width {
                std::u32::MAX => vk::Extent2D {
                    width: window.inner_size().width,
                    height: window.inner_size().height,
                },
                _ => surface_capabilities.current_extent,
            };

            // swapchain image count
            self.image_count = surface_capabilities.min_image_count + 1;
            if surface_capabilities.max_image_count > 0
                && self.image_count > surface_capabilities.max_image_count
            {
                self.image_count = surface_capabilities.max_image_count;
            }

            // pre-transform
            let pre_transform = if surface_capabilities
                .supported_transforms
                .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
            {
                vk::SurfaceTransformFlagsKHR::IDENTITY
            } else {
                surface_capabilities.current_transform
            };

            let swapchain_ci = vk::SwapchainCreateInfoKHR::builder()
                .surface(*self.surface)
                .min_image_count(self.image_count)
                .image_color_space(self.format.color_space)
                .image_format(self.format.format)
                .image_extent(self.resolution)
                .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                .pre_transform(pre_transform)
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                .present_mode(present_mode)
                .clipped(true)
                .image_array_layers(1);
            self.swapchain = self
                .swapchain_loader
                .create_swapchain(&swapchain_ci, None)
                .unwrap();

            // swapchain image views
            let swapchain_images = self
                .swapchain_loader
                .get_swapchain_images(self.swapchain)
                .unwrap();
            self.swapchain_image_views = swapchain_images
                .iter()
                .map(|&image| {
                    let create_view_info = vk::ImageViewCreateInfo::builder()
                        .view_type(vk::ImageViewType::TYPE_2D)
                        .format(self.format.format)
                        .components(vk::ComponentMapping {
                            r: vk::ComponentSwizzle::R,
                            g: vk::ComponentSwizzle::G,
                            b: vk::ComponentSwizzle::B,
                            a: vk::ComponentSwizzle::A,
                        })
                        .subresource_range(vk::ImageSubresourceRange {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            base_mip_level: 0,
                            level_count: 1,
                            base_array_layer: 0,
                            layer_count: 1,
                        })
                        .image(image);
                    self.device
                        .create_image_view(&create_view_info, None)
                        .unwrap()
                })
                .collect();
        }
    }

    /// Destroy vulkan objects owned by this struct
    fn destroy_vk_objs(&mut self) {
        todo!("device check");
        unsafe {
            for &image_view in self.swapchain_image_views.iter() {
                self.device.destroy_image_view(image_view, None);
            }
            if self.swapchain != vk::SwapchainKHR::default() {
                self.swapchain_loader
                    .destroy_swapchain(self.swapchain, None);
            }
        }
    }

    /// Set default values for non-immutable fields
    fn set_defaults(&mut self) {
        self.swapchain = vk::SwapchainKHR::default();
        self.swapchain_image_views = Vec::default();
        self.format = vk::SurfaceFormatKHR::default();
        self.resolution = vk::Extent2D::default();
        self.image_count = u32::default();
    }
}

impl Drop for PresentTarget {
    fn drop(&mut self) {
        self.destroy_vk_objs();
    }
}
