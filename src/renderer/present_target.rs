pub use ash::{Device, Instance};
use ash::{
	extensions::khr::{Surface, Swapchain},
	vk,
};

pub struct PresentTarget {
	// todo weak device pointer
	device: Device,

	image_count: u32,
	dimensions: vk::Extent2D,
	work_group_count: [u32; 2],

	swapchain_loader: Swapchain,
	swapchain: vk::SwapchainKHR,
	swapchain_image_views: Vec<vk::ImageView>,
}

impl PresentTarget {
	pub fn new(instance: Instance, device: Device,
		physical_device: vk::PhysicalDevice, surface_loader: Surface,
		surface: vk::SurfaceKHR, requested_width: u32, requested_height: u32) -> Self {
		unsafe {
			// surface capabilities
			let surface_capabilities = surface_loader
				.get_physical_device_surface_capabilities(physical_device, surface)
				.unwrap();
			let surface_format = surface_loader
				.get_physical_device_surface_formats(physical_device, surface)
				.unwrap()[0];
			let surface_resolution = match surface_capabilities.current_extent.width {
				std::u32::MAX => vk::Extent2D {
					width: requested_width,
					height: requested_height,
				},
				_ => surface_capabilities.current_extent,
			};

			let mut image_count = surface_capabilities.min_image_count + 1;
			if surface_capabilities.max_image_count > 0
				&& image_count > surface_capabilities.max_image_count
			{
				image_count = surface_capabilities.max_image_count;
			}
			let pre_transform = if surface_capabilities
				.supported_transforms
				.contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
			{
				vk::SurfaceTransformFlagsKHR::IDENTITY
			} else {
				surface_capabilities.current_transform
			};
			let present_modes = surface_loader
				.get_physical_device_surface_present_modes(physical_device, surface)
				.unwrap();
			let present_mode = present_modes
				.iter()
				.cloned()
				.find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
				.unwrap_or(vk::PresentModeKHR::FIFO);

			let mut present_target = PresentTarget {
				device: device,
				swapchain_loader: Swapchain::new(&instance, &device),
				dimensions: vk::Extent2D::default(),
				work_group_count: [0, 0],
				swapchain: vk::SwapchainKHR::default(),
				swapchain_image_views: Vec::default(),
			};
			present_target.create();
			present_target
		}
	}
	
	pub fn recreate(&mut self) {
		self.cleanup();
		self.create();
	}

	fn create(&mut self) {
		let swapchain_ci = vk::SwapchainCreateInfoKHR::builder()
			.surface(surface)
			.min_image_count(image_count)
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

		let swapchain =self.swapchain_loader
			.create_swapchain(&swapchain_ci, None)
			.unwrap()

		// swapchain image views
		let swapchain_images = swapchain_loader.get_swapchain_images(swapchain).unwrap();
		let swapchain_image_views: Vec<vk::ImageView> = swapchain_images
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
				self.device.create_image_view(&create_view_info, None).unwrap()
			})
			.collect();
	}

	fn cleanup(&mut self) {
		unsafe {
			for &image_view in self.swapchain_image_views.iter() {
				self.device.destroy_image_view(image_view, None);
			}
			self.swapchain_loader
				.destroy_swapchain(self.swapchain, None);
		}
		self.dimensions = vk::Extent2D::default();
		self.work_group_count = [0, 0];
	}
}

impl Drop for PresentTarget {
	fn drop(&mut self) {
		self.cleanup();
		//todo
	}
}
