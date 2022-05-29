/*!
Abstraction for screen render target i.e. swapchain
*/

use ash::{extensions::khr::Swapchain, vk, Device, Instance};

pub struct Screen {
    pub swapchain_loader: Swapchain,
    pub swapchain: vk::SwapchainKHR,
    pub image_views: Vec<vk::ImageView>,
}

impl Screen {
    pub fn new(
        instance: &Instance,
        device: &Device,
        surface: vk::SurfaceKHR,
        surface_capabilities: vk::SurfaceCapabilitiesKHR,
        surface_format: vk::SurfaceFormatKHR,
        surface_resolution: vk::Extent2D,
        present_mode: vk::PresentModeKHR,
    ) -> Self {
        unsafe {
            let swapchain_loader = Swapchain::new(instance, device);
            let mut desired_image_count = surface_capabilities.min_image_count + 1;
            if surface_capabilities.max_image_count > 0
                && desired_image_count > surface_capabilities.max_image_count
            {
                desired_image_count = surface_capabilities.max_image_count;
            }
            let pre_transform = if surface_capabilities
                .supported_transforms
                .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
            {
                vk::SurfaceTransformFlagsKHR::IDENTITY
            } else {
                surface_capabilities.current_transform
            };

            let swapchain_ci = vk::SwapchainCreateInfoKHR::builder()
                .surface(surface)
                .min_image_count(desired_image_count)
                .image_color_space(surface_format.color_space)
                .image_format(surface_format.format)
                .image_extent(surface_resolution)
                .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                .pre_transform(pre_transform)
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                .present_mode(present_mode)
                .clipped(true)
                .image_array_layers(1);

            let swapchain = swapchain_loader
                .create_swapchain(&swapchain_ci, None)
                .unwrap();

            // swapchain image views
            let images = swapchain_loader.get_swapchain_images(swapchain).unwrap();
            let image_views: Vec<vk::ImageView> = images
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
                    device.create_image_view(&create_view_info, None).unwrap()
                })
                .collect();
            Self {
                swapchain_loader,
                swapchain,
                image_views,
            }
        }
    }

    pub fn destroy(&mut self, device: &Device) {
        unsafe {
            for &image_view in self.image_views.iter() {
                device.destroy_image_view(image_view, None);
            }
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
        }
    }
}
