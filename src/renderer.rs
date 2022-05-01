use ash::{
    extensions::{
        ext::DebugUtils,
        khr::{Surface, Swapchain},
    },
    util::*,
    vk, Entry,
};
pub use ash::{Device, Instance};
use std::{
    borrow::Cow, cell::RefCell, default::Default, ffi::CStr, io::Cursor, mem, mem::align_of,
    ops::Drop, os::raw::c_char,
};
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn,
    window::WindowBuilder,
};

// Simple offset_of macro akin to C++ offsetof
#[macro_export]
macro_rules! offset_of {
    ($self:path, $field:ident) => {{
        #[allow(unused_unsafe)]
        unsafe {
            let b: $self = mem::zeroed();
            (&b.$field as *const _ as isize) - (&b as *const _ as isize)
        }
    }};
}

/// Helper function for submitting command buffers. Immediately waits for the fence before the command buffer
/// is executed. That way we can delay the waiting for the fences by 1 frame which is good for performance.
/// Make sure to create the fence in a signaled state on the first use.
#[allow(clippy::too_many_arguments)]
pub fn record_submit_commandbuffer<F: FnOnce(&Device, vk::CommandBuffer)>(
    device: &Device,
    command_buffer: vk::CommandBuffer,
    command_buffer_reuse_fence: vk::Fence,
    submit_queue: vk::Queue,
    wait_mask: &[vk::PipelineStageFlags],
    wait_semaphores: &[vk::Semaphore],
    signal_semaphores: &[vk::Semaphore],
    f: F,
) {
    unsafe {
        device
            .wait_for_fences(&[command_buffer_reuse_fence], true, std::u64::MAX)
            .expect("Wait for fence failed.");

        device
            .reset_fences(&[command_buffer_reuse_fence])
            .expect("Reset fences failed.");

        device
            .reset_command_buffer(
                command_buffer,
                vk::CommandBufferResetFlags::RELEASE_RESOURCES,
            )
            .expect("Reset command buffer failed.");

        let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        device
            .begin_command_buffer(command_buffer, &command_buffer_begin_info)
            .expect("Begin commandbuffer");
        f(device, command_buffer);
        device
            .end_command_buffer(command_buffer)
            .expect("End commandbuffer");

        let command_buffers = vec![command_buffer];

        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(wait_semaphores)
            .wait_dst_stage_mask(wait_mask)
            .command_buffers(&command_buffers)
            .signal_semaphores(signal_semaphores);

        device
            .queue_submit(
                submit_queue,
                &[submit_info.build()],
                command_buffer_reuse_fence,
            )
            .expect("queue submit failed.");
    }
}

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;
    let message_id_number: i32 = callback_data.message_id_number as i32;

    let message_id_name = if callback_data.p_message_id_name.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
    };

    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    println!(
        "{:?}:\n{:?} [{} ({})] : {}\n",
        message_severity,
        message_type,
        message_id_name,
        &message_id_number.to_string(),
        message,
    );

    vk::FALSE
}

pub fn find_memorytype_index(
    memory_req: &vk::MemoryRequirements,
    memory_prop: &vk::PhysicalDeviceMemoryProperties,
    flags: vk::MemoryPropertyFlags,
) -> Option<u32> {
    memory_prop.memory_types[..memory_prop.memory_type_count as _]
        .iter()
        .enumerate()
        .find(|(index, memory_type)| {
            (1 << index) & memory_req.memory_type_bits != 0
                && memory_type.property_flags & flags == flags
        })
        .map(|(index, _memory_type)| index as _)
}

pub struct Renderer {
    entry: Entry,
    instance: Instance,
    device: Device,
    surface_loader: Surface,
    swapchain_loader: Swapchain,
    debug_utils_loader: DebugUtils,
    window: winit::window::Window,
    event_loop: RefCell<EventLoop<()>>,
    debug_call_back: vk::DebugUtilsMessengerEXT,

    pdevice: vk::PhysicalDevice,
    device_memory_properties: vk::PhysicalDeviceMemoryProperties,
    queue_family_index: u32,
    present_queue: vk::Queue,

    surface: vk::SurfaceKHR,
    surface_format: vk::SurfaceFormatKHR,
    surface_resolution: vk::Extent2D,
    work_group_count: [u32; 2],

    swapchain: vk::SwapchainKHR,
    swapchain_images: Vec<vk::Image>,
    swapchain_image_views: Vec<vk::ImageView>,

    command_pool: vk::CommandPool,
    command_buffer_render: vk::CommandBuffer,
    setup_command_buffer: vk::CommandBuffer,

    render_image: vk::Image,
    render_image_view: vk::ImageView,
    render_image_memory: vk::DeviceMemory,

    present_complete_semaphore: vk::Semaphore,
    rendering_complete_semaphore: vk::Semaphore,

    render_commands_reuse_fence: vk::Fence,
    setup_commands_reuse_fence: vk::Fence,

    descriptor_pool: vk::DescriptorPool,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_set: vk::DescriptorSet,

    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
}

impl Renderer {
    pub fn render_loop(&self) {
        self.event_loop
            .borrow_mut()
            .run_return(|event, _, control_flow| {
                *control_flow = ControlFlow::Poll;
                match event {
                    Event::WindowEvent {
                        event:
                            WindowEvent::CloseRequested
                            | WindowEvent::KeyboardInput {
                                input:
                                    KeyboardInput {
                                        state: ElementState::Pressed,
                                        virtual_keycode: Some(VirtualKeyCode::Escape),
                                        ..
                                    },
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    Event::MainEventsCleared => self.render_frame(),
                    _ => (),
                }
            });
    }

    fn render_frame(&self) {
        unsafe {
            // TODO semaphores

            // aquire next swapchain image
            let (present_index, _) = self
                .swapchain_loader
                .acquire_next_image(
                    self.swapchain,
                    std::u64::MAX,
                    self.present_complete_semaphore,
                    vk::Fence::null(),
                )
                .unwrap();

            record_submit_commandbuffer(
                &self.device,
                self.command_buffer_render,
                self.render_commands_reuse_fence,
                self.present_queue,
                &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT],
                &[self.present_complete_semaphore],
                &[self.rendering_complete_semaphore],
                |device, command_buffer_render| {
                    // transition render image layout from transfer src to general

                    device.cmd_pipeline_barrier(
                        command_buffer_render,
                        vk::PipelineStageFlags::TRANSFER,
                        vk::PipelineStageFlags::COMPUTE_SHADER,
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &[vk::ImageMemoryBarrier {
                            image: self.render_image,
                            old_layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                            new_layout: vk::ImageLayout::GENERAL,
                            src_access_mask: vk::AccessFlags::TRANSFER_READ,
                            dst_access_mask: vk::AccessFlags::SHADER_WRITE,
                            subresource_range: vk::ImageSubresourceRange {
                                aspect_mask: vk::ImageAspectFlags::COLOR,
                                layer_count: 1,
                                level_count: 1,
                                ..Default::default()
                            },
                            ..Default::default()
                        }],
                    );

                    // dispatch render

                    device.cmd_bind_pipeline(
                        command_buffer_render,
                        vk::PipelineBindPoint::COMPUTE,
                        self.pipeline,
                    );
                    device.cmd_bind_descriptor_sets(
                        command_buffer_render,
                        vk::PipelineBindPoint::COMPUTE,
                        self.pipeline_layout,
                        0,
                        &[self.descriptor_set],
                        &[],
                    );
                    device.cmd_dispatch(
                        command_buffer_render,
                        self.work_group_count[0],
                        self.work_group_count[1],
                        1,
                    );

                    // transition render image layout from general to transfer src
                    // transition swapchain image from present src to transfer dst

                    device.cmd_pipeline_barrier(
                        command_buffer_render,
                        vk::PipelineStageFlags::COMPUTE_SHADER,
                        vk::PipelineStageFlags::TRANSFER,
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &[
                            vk::ImageMemoryBarrier {
                                image: self.render_image,
                                old_layout: vk::ImageLayout::GENERAL,
                                new_layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                                src_access_mask: vk::AccessFlags::SHADER_READ,
                                dst_access_mask: vk::AccessFlags::TRANSFER_READ,
                                subresource_range: vk::ImageSubresourceRange {
                                    aspect_mask: vk::ImageAspectFlags::COLOR,
                                    layer_count: 1,
                                    level_count: 1,
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                            vk::ImageMemoryBarrier {
                                image: self.swapchain_images[present_index as usize],
                                old_layout: vk::ImageLayout::PRESENT_SRC_KHR,
                                new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                                src_access_mask: vk::AccessFlags::MEMORY_READ,
                                dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
                                subresource_range: vk::ImageSubresourceRange {
                                    aspect_mask: vk::ImageAspectFlags::COLOR,
                                    layer_count: 1,
                                    level_count: 1,
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                        ],
                    );

                    // blit render to swapchain image

                    let blit_size = vk::Offset3D {
                        x: self.surface_resolution.width as i32,
                        y: self.surface_resolution.height as i32,
                        z: 1,
                    };
                    device.cmd_blit_image(
                        command_buffer_render,
                        self.render_image,
                        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                        self.swapchain_images[present_index as usize],
                        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                        &[vk::ImageBlit {
                            src_subresource: vk::ImageSubresourceLayers {
                                aspect_mask: vk::ImageAspectFlags::COLOR,
                                layer_count: 1,
                                ..Default::default()
                            },
                            dst_subresource: vk::ImageSubresourceLayers {
                                aspect_mask: vk::ImageAspectFlags::COLOR,
                                layer_count: 1,
                                ..Default::default()
                            },
                            src_offsets: [Default::default(), blit_size],
                            dst_offsets: [Default::default(), blit_size],
                        }],
                        vk::Filter::LINEAR,
                    );

                    // transfer swapchain image layout back to present src

                    device.cmd_pipeline_barrier(
                        command_buffer_render,
                        vk::PipelineStageFlags::TRANSFER,
                        vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &[vk::ImageMemoryBarrier {
                            image: self.swapchain_images[present_index as usize],
                            old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                            new_layout: vk::ImageLayout::PRESENT_SRC_KHR,
                            src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
                            dst_access_mask: vk::AccessFlags::MEMORY_READ,
                            subresource_range: vk::ImageSubresourceRange {
                                aspect_mask: vk::ImageAspectFlags::COLOR,
                                layer_count: 1,
                                level_count: 1,
                                ..Default::default()
                            },
                            ..Default::default()
                        }],
                    );
                },
            );

            // present swapchain image
            let wait_semaphors = [self.rendering_complete_semaphore];
            let swapchains = [self.swapchain];
            let image_indices = [present_index];
            let present_info = vk::PresentInfoKHR::builder()
                .wait_semaphores(&wait_semaphors) // &self.rendering_complete_semaphore)
                .swapchains(&swapchains)
                .image_indices(&image_indices);
            self.swapchain_loader
                .queue_present(self.present_queue, &present_info)
                .unwrap();
        }
    }

    pub fn new(window_width: u32, window_height: u32) -> Self {
        unsafe {
            // winit window

            let event_loop = EventLoop::new();
            let window = WindowBuilder::new()
                .with_title("Ash - Example")
                .with_inner_size(winit::dpi::LogicalSize::new(
                    f64::from(window_width),
                    f64::from(window_height),
                ))
                .build(&event_loop)
                .unwrap();
            let entry = Entry::linked();
            let app_name = CStr::from_bytes_with_nul_unchecked(b"VulkanTriangle\0");

            // vulkan layers

            let layer_names = [CStr::from_bytes_with_nul_unchecked(
                b"VK_LAYER_KHRONOS_validation\0",
            )];
            let layers_names_raw: Vec<*const c_char> = layer_names
                .iter()
                .map(|raw_name| raw_name.as_ptr())
                .collect();

            // surface extensins

            let mut extension_names = ash_window::enumerate_required_extensions(&window)
                .unwrap()
                .to_vec();
            extension_names.push(DebugUtils::name().as_ptr());

            // create instance

            let appinfo = vk::ApplicationInfo::builder()
                .application_name(app_name)
                .application_version(0)
                .engine_name(app_name)
                .engine_version(0)
                .api_version(vk::make_api_version(0, 1, 0, 0));
            let create_info = vk::InstanceCreateInfo::builder()
                .application_info(&appinfo)
                .enabled_layer_names(&layers_names_raw)
                .enabled_extension_names(&extension_names);

            let instance: Instance = entry
                .create_instance(&create_info, None)
                .expect("Instance creation error");

            // debug callback

            let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
                .message_severity(
                    vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                        | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                        | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
                )
                .message_type(
                    vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                        | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                        | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                )
                .pfn_user_callback(Some(vulkan_debug_callback));

            let debug_utils_loader = DebugUtils::new(&entry, &instance);
            let debug_call_back = debug_utils_loader
                .create_debug_utils_messenger(&debug_info, None)
                .unwrap();

            // create surface

            let surface = ash_window::create_surface(&entry, &instance, &window, None).unwrap();

            // choose physical device and queue family

            let pdevices = instance
                .enumerate_physical_devices()
                .expect("Physical device error");
            let surface_loader = Surface::new(&entry, &instance);
            let (pdevice, queue_family_index) = pdevices
                .iter()
                .map(|pdevice| {
                    instance
                        .get_physical_device_queue_family_properties(*pdevice)
                        .iter()
                        .enumerate()
                        .filter_map(|(index, info)| {
                            let supports_graphic_and_surface =
                                info.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                                    && surface_loader
                                        .get_physical_device_surface_support(
                                            *pdevice,
                                            index as u32,
                                            surface,
                                        )
                                        .unwrap();
                            if supports_graphic_and_surface {
                                Some((*pdevice, index))
                            } else {
                                None
                            }
                        })
                        .next()
                })
                .flatten()
                .next()
                .expect("Couldn't find suitable device.");

            let queue_family_index = queue_family_index as u32;
            let priorities = [1.0];
            let queue_info = vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(queue_family_index)
                .queue_priorities(&priorities);

            // get physical device memory properties

            let device_memory_properties = instance.get_physical_device_memory_properties(pdevice);

            // create device

            let device_extension_names_raw = [Swapchain::name().as_ptr()];
            let features = vk::PhysicalDeviceFeatures {
                shader_clip_distance: 1,
                ..Default::default()
            };

            let device_create_info = vk::DeviceCreateInfo::builder()
                .queue_create_infos(std::slice::from_ref(&queue_info))
                .enabled_extension_names(&device_extension_names_raw)
                .enabled_features(&features);

            let device: Device = instance
                .create_device(pdevice, &device_create_info, None)
                .unwrap();

            // rendering queue

            let present_queue = device.get_device_queue(queue_family_index as u32, 0);

            // check for swapchain blit support and choose swapchain image format accordingly

            let surface_capabilities = surface_loader
                .get_physical_device_surface_capabilities(pdevice, surface)
                .unwrap();
            if !surface_capabilities
                .supported_usage_flags
                .contains(vk::ImageUsageFlags::TRANSFER_DST)
            {
                panic!("Surface doesn't support transfer dst image usage flag")
            }
            let surface_format =
                Renderer::choose_surface_format(instance, surface_loader, surface, pdevice)
                    .expect("{:?}");

            // create swapchain

            let mut desired_image_count = surface_capabilities.min_image_count + 1;
            if surface_capabilities.max_image_count > 0
                && desired_image_count > surface_capabilities.max_image_count
            {
                desired_image_count = surface_capabilities.max_image_count;
            }
            let surface_resolution = match surface_capabilities.current_extent.width {
                std::u32::MAX => vk::Extent2D {
                    width: window_width,
                    height: window_height,
                },
                _ => surface_capabilities.current_extent,
            };
            let pre_transform = if surface_capabilities
                .supported_transforms
                .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
            {
                vk::SurfaceTransformFlagsKHR::IDENTITY
            } else {
                surface_capabilities.current_transform
            };
            let present_modes = surface_loader
                .get_physical_device_surface_present_modes(pdevice, surface)
                .unwrap();
            let present_mode = present_modes
                .iter()
                .cloned()
                .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
                .unwrap_or(vk::PresentModeKHR::FIFO);
            let swapchain_loader = Swapchain::new(&instance, &device);

            let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
                .surface(surface)
                .min_image_count(desired_image_count)
                .image_color_space(surface_format.color_space)
                .image_format(surface_format.format)
                .image_extent(surface_resolution)
                .image_usage(
                    vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST,
                )
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                .pre_transform(pre_transform)
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                .present_mode(present_mode)
                .clipped(true)
                .image_array_layers(1);

            let swapchain = swapchain_loader
                .create_swapchain(&swapchain_create_info, None)
                .unwrap();

            // create command command_pool

            let pool_create_info = vk::CommandPoolCreateInfo::builder()
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                .queue_family_index(queue_family_index);

            let command_pool = device.create_command_pool(&pool_create_info, None).unwrap();

            // create command buffers

            let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
                .command_buffer_count(2)
                .command_pool(command_pool)
                .level(vk::CommandBufferLevel::PRIMARY);

            let command_buffers = device
                .allocate_command_buffers(&command_buffer_allocate_info)
                .unwrap();
            let setup_command_buffer = command_buffers[0];
            let command_buffer_render = command_buffers[1];

            // create present image views

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
                    device.create_image_view(&create_view_info, None).unwrap()
                })
                .collect();

            // create render image

            let render_image_create_info = vk::ImageCreateInfo::builder()
                .image_type(vk::ImageType::TYPE_2D)
                .format(vk::Format::R8G8B8A8_UNORM)
                .extent(surface_resolution.into())
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);

            let render_image = device
                .create_image(&render_image_create_info, None)
                .unwrap();

            // allocate memory for render image

            let render_image_memory_req = device.get_image_memory_requirements(render_image);
            let render_image_memory_index = find_memorytype_index(
                &render_image_memory_req,
                &device_memory_properties,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            )
            .expect("Unable to find suitable memory index for render image.");

            let render_image_allocate_info = vk::MemoryAllocateInfo::builder()
                .allocation_size(render_image_memory_req.size)
                .memory_type_index(render_image_memory_index);

            let render_image_memory = device
                .allocate_memory(&render_image_allocate_info, None)
                .unwrap();

            device
                .bind_image_memory(render_image, render_image_memory, 0)
                .expect("Unable to bind render image memory");

            // create setup and render fences

            let fence_create_info =
                vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);

            let setup_commands_reuse_fence = device
                .create_fence(&fence_create_info, None)
                .expect("Create fence failed.");
            let render_commands_reuse_fence = device
                .create_fence(&fence_create_info, None)
                .expect("Create fence failed.");

            // transition image layout for render image

            record_submit_commandbuffer(
                &device,
                setup_command_buffer,
                setup_commands_reuse_fence,
                present_queue,
                &[],
                &[],
                &[],
                |device, setup_command_buffer| {
                    let layout_transition_barrier = vk::ImageMemoryBarrier {
                        image: render_image,
                        old_layout: vk::ImageLayout::UNDEFINED,
                        new_layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                        src_access_mask: vk::AccessFlags::NONE,
                        dst_access_mask: vk::AccessFlags::TRANSFER_READ,
                        subresource_range: vk::ImageSubresourceRange {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            layer_count: 1,
                            level_count: 1,
                            ..Default::default()
                        },
                        ..Default::default()
                    };

                    device.cmd_pipeline_barrier(
                        setup_command_buffer,
                        vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                        vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &[layout_transition_barrier],
                    );
                },
            );

            // crete render image view

            let render_image_view_info = vk::ImageViewCreateInfo::builder()
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    layer_count: 1,
                    level_count: 1,
                    ..Default::default()
                })
                .image(render_image)
                .format(render_image_create_info.format)
                .view_type(vk::ImageViewType::TYPE_2D);

            let render_image_view = device
                .create_image_view(&render_image_view_info, None)
                .unwrap();

            // create descriptor pool

            let pool_sizes = [vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_IMAGE,
                descriptor_count: 1,
                ..Default::default()
            }];
            let max_sets = 1u32;
            let descriptor_pool = device
                .create_descriptor_pool(
                    &vk::DescriptorPoolCreateInfo::builder()
                        .pool_sizes(&pool_sizes)
                        .max_sets(max_sets)
                        .build(),
                    None,
                )
                .expect("Failed to create descriptor pool");

            // descriptor set layout

            let descriptor_bindings = [vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE)
                .build()];
            let descriptor_set_layout = device
                .create_descriptor_set_layout(
                    &vk::DescriptorSetLayoutCreateInfo::builder()
                        .bindings(&descriptor_bindings)
                        .build(),
                    None,
                )
                .expect("Failed to create descriptor set layout");

            // create descriptor set

            let descriptor_set = device
                .allocate_descriptor_sets(
                    &vk::DescriptorSetAllocateInfo::builder()
                        .descriptor_pool(descriptor_pool)
                        .set_layouts(&[descriptor_set_layout])
                        .build(),
                )
                .expect("Failed to create descriptor set")[0];

            // write descriptor set

            let descriptor_set_write = vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                .image_info(&[vk::DescriptorImageInfo {
                    image_view: render_image_view,
                    image_layout: vk::ImageLayout::GENERAL,
                    sampler: vk::Sampler::default(), // unused
                }])
                .build();

            // load compute shader code

            let mut render_spv_file =
                Cursor::new(&include_bytes!("../assets/shader_binaries/render.comp.spv")[..]);
            let render_code = read_spv(&mut render_spv_file)
                .expect("Failed to read render compute shader spirv file");
            let render_shader_module = device
                .create_shader_module(
                    &vk::ShaderModuleCreateInfo::builder().code(&render_code),
                    None,
                )
                .expect("Failed to create shader module");

            // create compute pipeline

            let pipeline_layout = device
                .create_pipeline_layout(
                    &vk::PipelineLayoutCreateInfo::builder()
                        .set_layouts(&[descriptor_set_layout])
                        .build(),
                    None,
                )
                .expect("Failed to create pipeline layout");

            let pipeline_create_info = vk::ComputePipelineCreateInfo::builder()
                .stage(
                    vk::PipelineShaderStageCreateInfo::builder()
                        .stage(vk::ShaderStageFlags::COMPUTE)
                        .module(render_shader_module)
                        .name(CStr::from_bytes_with_nul_unchecked(b"main\0"))
                        .build(),
                )
                .build();
            let pipeline = device
                .create_compute_pipelines(vk::PipelineCache::null(), &[pipeline_create_info], None)
                .expect("failed to create compute pipeline")[0];

            // cleanup shader module

            device.destroy_shader_module(render_shader_module, None);

            // calculate compute dispatch work group count

            let work_group_count = Renderer::calc_work_group_count(surface_resolution.into());

            // create semaphores for completion of presenting and rendering

            let semaphore_create_info = vk::SemaphoreCreateInfo::default();

            let present_complete_semaphore = device
                .create_semaphore(&semaphore_create_info, None)
                .unwrap();
            let rendering_complete_semaphore = device
                .create_semaphore(&semaphore_create_info, None)
                .unwrap();

            // return self

            Renderer {
                event_loop: RefCell::new(event_loop),
                entry,
                instance,
                device,
                queue_family_index,
                pdevice,
                device_memory_properties,
                window,
                surface_loader,
                surface_format,
                present_queue,
                surface_resolution,
                work_group_count,
                swapchain_loader,
                swapchain,
                swapchain_images,
                swapchain_image_views,
                command_pool,
                command_buffer_render,
                setup_command_buffer,
                render_image,
                render_image_view,
                present_complete_semaphore,
                rendering_complete_semaphore,
                render_commands_reuse_fence,
                setup_commands_reuse_fence,
                surface,
                debug_call_back,
                debug_utils_loader,
                render_image_memory,
                descriptor_pool,
                descriptor_set_layout,
                descriptor_set,
                pipeline_layout,
                pipeline,
            }
        }
    }

    // TODO do this during the physical device choosing loop
    fn choose_surface_format(
        instance: Instance,
        surface_loader: Surface,
        surface: vk::SurfaceKHR,
        physical_device: vk::PhysicalDevice,
    ) -> Result<vk::SurfaceFormatKHR, String> {
        unsafe {
            let surface_formats = surface_loader
                .get_physical_device_surface_formats(physical_device, surface)
                .unwrap();
            // TODO functional version (for fun)
            for surface_format in surface_formats.into_iter() {
                let format_props = instance
                    .get_physical_device_format_properties(physical_device, surface_format.format);
                if format_props
                    .optimal_tiling_features
                    .contains(vk::FormatFeatureFlags::BLIT_DST)
                {
                    return Ok(surface_format);
                }
            }
            Err(String::from("Swapchain does not support dst blit"))
        }
    }

    fn calc_work_group_count(image_size: vk::Extent2D) -> [u32; 2] {
        let group_count_x = image_size.width / 16;
        if (image_size.width % 16) != 0 {
            group_count_x = group_count_x + 1;
        }
        let group_count_y = image_size.height / 16;
        if (image_size.height % 16) != 0 {
            group_count_y = group_count_y + 1;
        }
        [group_count_x, group_count_y]
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();
            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.destroy_pipeline(self.pipeline, None);
            self.device
                .destroy_semaphore(self.present_complete_semaphore, None);
            self.device
                .destroy_semaphore(self.rendering_complete_semaphore, None);
            self.device
                .destroy_fence(self.render_commands_reuse_fence, None);
            self.device
                .destroy_fence(self.setup_commands_reuse_fence, None);
            self.device.free_memory(self.render_image_memory, None);
            self.device.destroy_image_view(self.render_image_view, None);
            self.device.destroy_image(self.render_image, None);
            for &image_view in self.swapchain_image_views.iter() {
                self.device.destroy_image_view(image_view, None);
            }
            self.device.destroy_command_pool(self.command_pool, None);
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.debug_utils_loader
                .destroy_debug_utils_messenger(self.debug_call_back, None);
            self.instance.destroy_instance(None);
        }
    }
}
