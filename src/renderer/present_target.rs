// todo commenting
// todo unwraps

use ash::{
    extensions::khr::{Surface, Swapchain},
    vk,
};
pub use ash::{Device, Instance};
use winit::window::Window;

fn calc_work_group_count(image_size: vk::Extent2D) -> [u32; 2] {
    let mut group_count_x = image_size.width / 16;
    if (image_size.width % 16) != 0 {
        group_count_x = group_count_x + 1;
    }
    let mut group_count_y = image_size.height / 16;
    if (image_size.height % 16) != 0 {
        group_count_y = group_count_y + 1;
    }
    [group_count_x, group_count_y]
}

pub struct PresentTarget {
    // todo weak device pointers?
    // References
    device: Device,
    physical_device: vk::PhysicalDevice,
    surface_loader: Surface,
    surface: vk::SurfaceKHR,

    // todo getters!
    pub swapchain_loader: Swapchain, // (doesn't impl Default unfortunately)
    pub swapchain: vk::SwapchainKHR,
    pub swapchain_image_views: Vec<vk::ImageView>,

    pub image_count: u32,
    pub resolution: vk::Extent2D,
    pub work_group_count: [u32; 2],
}

impl PresentTarget {
    pub fn new(
        window: &Window,
        instance: Instance,
        device: Device,
        physical_device: vk::PhysicalDevice,
        surface_loader: Surface,
        surface: vk::SurfaceKHR,
    ) -> Self {
        let mut present_target = PresentTarget {
            device,
            physical_device,
            surface_loader,
            surface,
            swapchain_loader: Swapchain::new(&instance, &device),
            /* set in create() */ image_count: u32::default(),
            /* set in create() */ resolution: vk::Extent2D::default(),
            /* set in create() */ work_group_count: [u32::default(); 2],
            /* set in create() */ swapchain: vk::SwapchainKHR::default(),
            /* set in create() */ swapchain_image_views: Vec::default(),
        };
        unsafe {
            present_target.create(window);
        }
        present_target
    }

    pub fn recreate(&mut self, window: &Window) {
        self.cleanup();
        self.create(window);
    }

    fn create(&mut self, window: &Window) {
        unsafe {
            // present mode
            let present_mode = self
                .surface_loader
                .get_physical_device_surface_present_modes(self.physical_device, self.surface)
                .unwrap()
                .iter()
                .cloned()
                .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
                .unwrap_or(vk::PresentModeKHR::FIFO);

            // choose format todo preference
            let surface_format = self
                .surface_loader
                .get_physical_device_surface_formats(self.physical_device, self.surface)
                .unwrap()[0];

            // use surface capabilities data
            let surface_capabilities = self
                .surface_loader
                .get_physical_device_surface_capabilities(self.physical_device, self.surface)
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
                .surface(self.surface)
                .min_image_count(self.image_count)
                .image_color_space(surface_format.color_space)
                .image_format(surface_format.format)
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
                        .format(surface_format.format)
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

    fn cleanup(&mut self) {
        //todo device checkz
        unsafe {
            for &image_view in self.swapchain_image_views.iter() {
                self.device.destroy_image_view(image_view, None);
            }
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
        }
        self.resolution = vk::Extent2D::default();
        self.work_group_count = [0, 0];
    }
}

impl Drop for PresentTarget {
    fn drop(&mut self) {
        self.cleanup();
        //todo
    }
}
