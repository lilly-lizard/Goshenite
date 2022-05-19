use ash::{
    extensions::{
        ext::DebugUtils,
        khr::{Surface, Swapchain},
    },
    util::*,
    vk, Entry,
};
pub use ash::{Device, Instance};
use colored::Colorize;
use std::{borrow::Cow, default::Default, ffi::CStr, io::Cursor, ops::Drop, os::raw::c_char};
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn,
    window::Window,
};

// Config
static ENGINE_NAME: &[u8] = b"Goshenite\0";
static ENGINE_VER: u32 = 1;
static VULKAN_VER_MAJ: u32 = 2;
static VULKAN_VER_MIN: u32 = 0;

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;

    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };
    let message_severity_str = match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => "ERROR:".red(),
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => "WARNING:".yellow(),
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => "INFO:".blue(),
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => "VERBOSE:".magenta(),
        _ => "...".normal(),
    };

    println!(
        "{} {} {}",
        message_severity_str,
        format!("(Vulkan {:?})", message_type).dimmed(),
        if message_severity == vk::DebugUtilsMessageSeverityFlagsEXT::ERROR {
            message.bright_red()
        } else {
            message.normal()
        }
    );
    // break in debug builds
    debug_assert!(
        message_severity != vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
        "vulkan validation error (see log)",
    );

    vk::FALSE
}

pub struct Renderer {
    entry: Entry,
    instance: Instance,
    device: Device,
    surface_loader: Surface,
    swapchain_loader: Swapchain,

    debug_utils_loader: DebugUtils,
    debug_call_back: vk::DebugUtilsMessengerEXT,

    surface: vk::SurfaceKHR,
    surface_resolution: vk::Extent2D,
    work_group_count: [u32; 2],

    swapchain: vk::SwapchainKHR,
    swapchain_image_views: Vec<vk::ImageView>,

    queue: vk::Queue,

    command_pool: vk::CommandPool,
    command_buffer_render: vk::CommandBuffer,

    render_image: vk::Image,
    render_image_view: vk::ImageView,
    render_image_memory: vk::DeviceMemory,
    render_sampler: vk::Sampler,

    semaphore_present_complete: vk::Semaphore,
    semaphore_rendering_complete: vk::Semaphore,

    fence_render_commands_reuse: vk::Fence,
    fence_setup_commands_reuse: vk::Fence,

    descriptor_pool: vk::DescriptorPool,
    descriptor_set_layout_compute: vk::DescriptorSetLayout,
    descriptor_set_layout_post: vk::DescriptorSetLayout,
    descriptor_set_compute: vk::DescriptorSet,
    descriptor_set_post: vk::DescriptorSet,

    pipeline_layout_compute: vk::PipelineLayout,
    pipeline_layout_post: vk::PipelineLayout,
    pipeline_compute: vk::Pipeline,
    pipeline_post: vk::Pipeline,

    renderpass: vk::RenderPass,
    framebuffers: Vec<vk::Framebuffer>,
}

impl Renderer {
    pub fn render_loop(&self, event_loop: &mut EventLoop<()>) {
        event_loop.run_return(|event, _, control_flow| {
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
        unsafe {
            self.device.device_wait_idle().unwrap();
        }
    }

    fn render_frame(&self) {
        let clear_value = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 0.0],
            },
        }];
        let viewports = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: self.surface_resolution.width as f32,
            height: self.surface_resolution.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];
        let scissors = [self.surface_resolution.into()];

        unsafe {
            // aquire next swapchain image
            let (present_index, _) = self
                .swapchain_loader
                .acquire_next_image(
                    self.swapchain,
                    std::u64::MAX,
                    self.semaphore_present_complete,
                    vk::Fence::null(),
                )
                .unwrap();

            let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
                .render_pass(self.renderpass)
                .framebuffer(self.framebuffers[present_index as usize])
                .render_area(self.surface_resolution.into())
                .clear_values(&clear_value);

            record_submit_commandbuffer(
                &self.device,
                self.command_buffer_render,
                self.fence_render_commands_reuse,
                self.queue,
                &[vk::PipelineStageFlags::COMPUTE_SHADER],
                &[self.semaphore_present_complete],
                &[self.semaphore_rendering_complete],
                |device, command_buffer_render| {
                    // transition render image layout from shader read to general

                    device.cmd_pipeline_barrier(
                        command_buffer_render,
                        vk::PipelineStageFlags::FRAGMENT_SHADER,
                        vk::PipelineStageFlags::COMPUTE_SHADER,
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &[vk::ImageMemoryBarrier {
                            image: self.render_image,
                            old_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                            new_layout: vk::ImageLayout::GENERAL,
                            src_access_mask: vk::AccessFlags::SHADER_READ,
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
                        self.pipeline_compute,
                    );
                    device.cmd_bind_descriptor_sets(
                        command_buffer_render,
                        vk::PipelineBindPoint::COMPUTE,
                        self.pipeline_layout_compute,
                        0,
                        &[self.descriptor_set_compute],
                        &[],
                    );
                    device.cmd_dispatch(
                        command_buffer_render,
                        self.work_group_count[0],
                        self.work_group_count[1],
                        1,
                    );

                    // transition render image layout from general to shader read

                    device.cmd_pipeline_barrier(
                        command_buffer_render,
                        vk::PipelineStageFlags::COMPUTE_SHADER,
                        vk::PipelineStageFlags::FRAGMENT_SHADER,
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &[vk::ImageMemoryBarrier {
                            image: self.render_image,
                            old_layout: vk::ImageLayout::GENERAL,
                            new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                            src_access_mask: vk::AccessFlags::SHADER_WRITE,
                            dst_access_mask: vk::AccessFlags::SHADER_READ,
                            subresource_range: vk::ImageSubresourceRange {
                                aspect_mask: vk::ImageAspectFlags::COLOR,
                                layer_count: 1,
                                level_count: 1,
                                ..Default::default()
                            },
                            ..Default::default()
                        }],
                    );

                    // post processing

                    device.cmd_begin_render_pass(
                        command_buffer_render,
                        &render_pass_begin_info,
                        vk::SubpassContents::INLINE,
                    );

                    device.cmd_bind_pipeline(
                        command_buffer_render,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.pipeline_post,
                    );
                    device.cmd_bind_descriptor_sets(
                        command_buffer_render,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.pipeline_layout_post,
                        0,
                        &[self.descriptor_set_post],
                        &[],
                    );
                    device.cmd_set_viewport(command_buffer_render, 0, &viewports);
                    device.cmd_set_scissor(command_buffer_render, 0, &scissors);
                    device.cmd_draw(command_buffer_render, 3, 1, 0, 0);

                    device.cmd_end_render_pass(command_buffer_render);
                },
            );

            // present swapchain image
            let present_info = vk::PresentInfoKHR::builder()
                .wait_semaphores(&[self.semaphore_rendering_complete])
                .swapchains(&[self.swapchain])
                .image_indices(&[present_index])
                .build();
            self.swapchain_loader
                .queue_present(self.queue, &present_info)
                .unwrap();
        }
    }

    pub fn new(window: &Window, window_width: u32, window_height: u32) -> Self {
        // TODO pass as argument
        let app_name = CStr::from_bytes_with_nul(b"Goshenite\0").unwrap();
        let app_ver: u32 = 0;

        // vulkan entry point (for non-instance functions)
        let entry = Entry::linked();

        unsafe {
            // instance
            let instance = {
                // layers
                let layer_names = [CStr::from_bytes_with_nul_unchecked(
                    b"VK_LAYER_KHRONOS_validation\0",
                )];
                let layers_names_raw: Vec<*const c_char> = layer_names
                    .iter()
                    .map(|raw_name| raw_name.as_ptr())
                    .collect();

                // extensins
                let mut extension_names = ash_window::enumerate_required_extensions(&window)
                    .unwrap()
                    .to_vec();
                extension_names.push(DebugUtils::name().as_ptr());

                let appinfo = vk::ApplicationInfo::builder()
                    .application_name(app_name)
                    .application_version(app_ver)
                    .engine_name(CStr::from_bytes_with_nul_unchecked(ENGINE_NAME))
                    .engine_version(ENGINE_VER)
                    .api_version(vk::make_api_version(0, VULKAN_VER_MAJ, VULKAN_VER_MIN, 0));
                let instance_ci = vk::InstanceCreateInfo::builder()
                    .application_info(&appinfo)
                    .enabled_layer_names(&layers_names_raw)
                    .enabled_extension_names(&extension_names);

                // create
                entry
                    .create_instance(&instance_ci, None)
                    .expect("Instance creation error")
            };

            // debug callback
            let debug_utils_loader = DebugUtils::new(&entry, &instance);
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
            let debug_call_back = debug_utils_loader
                .create_debug_utils_messenger(&debug_info, None)
                .unwrap();

            // surface
            let surface = ash_window::create_surface(&entry, &instance, &window, None).unwrap();
            let surface_loader = Surface::new(&entry, &instance);

            // choose physical device and queue family
            let physical_devices = instance
                .enumerate_physical_devices()
                .expect("Physical device error");
            let (physical_device, queue_family_index) = physical_devices
                .iter()
                .map(|physical_device| {
                    instance
                        .get_physical_device_queue_family_properties(*physical_device)
                        .iter()
                        .enumerate()
                        .filter_map(|(index, info)| {
                            let supports_graphic_and_surface =
                                info.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                                    && surface_loader
                                        .get_physical_device_surface_support(
                                            *physical_device,
                                            index as u32,
                                            surface,
                                        )
                                        .unwrap();
                            if supports_graphic_and_surface {
                                Some((*physical_device, index as u32))
                            } else {
                                None
                            }
                        })
                        .next()
                })
                .flatten()
                .next()
                .expect("Couldn't find suitable device.");

            // get physical device memory properties
            let device_memory_properties =
                instance.get_physical_device_memory_properties(physical_device);

            // device
            let device = {
                let priorities = [1.0];
                let queue_info = vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(queue_family_index)
                    .queue_priorities(&priorities);

                let device_extension_names_raw = [Swapchain::name().as_ptr()];
                let features = vk::PhysicalDeviceFeatures {
                    shader_clip_distance: 1,
                    ..Default::default()
                };

                let device_ci = vk::DeviceCreateInfo::builder()
                    .queue_create_infos(std::slice::from_ref(&queue_info))
                    .enabled_extension_names(&device_extension_names_raw)
                    .enabled_features(&features);

                instance
                    .create_device(physical_device, &device_ci, None)
                    .unwrap()
            };

            // rendering queue
            let queue = device.get_device_queue(queue_family_index as u32, 0);

            // surface capabilities
            let surface_capabilities = surface_loader
                .get_physical_device_surface_capabilities(physical_device, surface)
                .unwrap();
            let surface_format = surface_loader
                .get_physical_device_surface_formats(physical_device, surface)
                .unwrap()[0];
            let surface_resolution = match surface_capabilities.current_extent.width {
                std::u32::MAX => vk::Extent2D {
                    width: window_width,
                    height: window_height,
                },
                _ => surface_capabilities.current_extent,
            };

            // swapchain
            let swapchain_loader = Swapchain::new(&instance, &device);
            let swapchain = {
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
                let present_modes = surface_loader
                    .get_physical_device_surface_present_modes(physical_device, surface)
                    .unwrap();
                let present_mode = present_modes
                    .iter()
                    .cloned()
                    .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
                    .unwrap_or(vk::PresentModeKHR::FIFO);

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

                swapchain_loader
                    .create_swapchain(&swapchain_ci, None)
                    .unwrap()
            };

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
                    device.create_image_view(&create_view_info, None).unwrap()
                })
                .collect();

            // create command command_pool
            let command_pool_ci = vk::CommandPoolCreateInfo::builder()
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                .queue_family_index(queue_family_index);
            let command_pool = device.create_command_pool(&command_pool_ci, None).unwrap();

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

            // create setup and render fences
            let fence_ci = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
            let fence_setup_commands_reuse = device
                .create_fence(&fence_ci, None)
                .expect("Create fence failed.");
            let fence_render_commands_reuse = device
                .create_fence(&fence_ci, None)
                .expect("Create fence failed.");

            // create semaphores for completion of presenting and rendering
            let semaphore_ci = vk::SemaphoreCreateInfo::default();
            let semaphore_present_complete = device.create_semaphore(&semaphore_ci, None).unwrap();
            let semaphore_rendering_complete =
                device.create_semaphore(&semaphore_ci, None).unwrap();

            // create render image
            let render_image_ci = vk::ImageCreateInfo::builder()
                .image_type(vk::ImageType::TYPE_2D)
                .format(vk::Format::R8G8B8A8_UNORM)
                .extent(surface_resolution.into())
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);

            let render_image = device.create_image(&render_image_ci, None).unwrap();

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

            // transition image layout for render image
            record_submit_commandbuffer(
                &device,
                setup_command_buffer,
                fence_setup_commands_reuse,
                queue,
                &[],
                &[],
                &[],
                |device, setup_command_buffer| {
                    let layout_transition_barrier = vk::ImageMemoryBarrier {
                        image: render_image,
                        old_layout: vk::ImageLayout::UNDEFINED,
                        new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                        src_access_mask: vk::AccessFlags::NONE,
                        dst_access_mask: vk::AccessFlags::SHADER_READ,
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
                        vk::PipelineStageFlags::FRAGMENT_SHADER,
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &[layout_transition_barrier],
                    );
                },
            );

            // render image view
            let render_image_view = {
                let render_image_view_info = vk::ImageViewCreateInfo::builder()
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        layer_count: 1,
                        level_count: 1,
                        ..Default::default()
                    })
                    .image(render_image)
                    .format(render_image_ci.format)
                    .view_type(vk::ImageViewType::TYPE_2D);

                device
                    .create_image_view(&render_image_view_info, None)
                    .unwrap()
            };

            // render sampler
            let render_sampler = device
                .create_sampler(
                    &vk::SamplerCreateInfo {
                        mag_filter: vk::Filter::LINEAR,
                        min_filter: vk::Filter::LINEAR,
                        mipmap_mode: vk::SamplerMipmapMode::LINEAR,
                        address_mode_u: vk::SamplerAddressMode::REPEAT,
                        address_mode_v: vk::SamplerAddressMode::REPEAT,
                        address_mode_w: vk::SamplerAddressMode::REPEAT,
                        max_anisotropy: 1.0,
                        border_color: vk::BorderColor::FLOAT_OPAQUE_WHITE,
                        compare_op: vk::CompareOp::NEVER,
                        ..Default::default()
                    },
                    None,
                )
                .expect("failed to create sampler");

            // descriptor pool
            let descriptor_pool = {
                let pool_sizes = [
                    vk::DescriptorPoolSize {
                        ty: vk::DescriptorType::STORAGE_IMAGE,
                        descriptor_count: 1,
                        ..Default::default()
                    },
                    vk::DescriptorPoolSize {
                        ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                        descriptor_count: 1,
                        ..Default::default()
                    },
                ];
                let max_sets = 2;
                device
                    .create_descriptor_pool(
                        &vk::DescriptorPoolCreateInfo::builder()
                            .pool_sizes(&pool_sizes)
                            .max_sets(max_sets),
                        None,
                    )
                    .expect("Failed to create descriptor pool")
            };

            // compute descriptor set
            let (descriptor_set_layout_compute, descriptor_set_compute) = {
                let descriptor_bindings = [vk::DescriptorSetLayoutBinding::builder()
                    .binding(0)
                    .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::COMPUTE)
                    .build()];
                let descriptor_set_layout = device
                    .create_descriptor_set_layout(
                        &vk::DescriptorSetLayoutCreateInfo::builder()
                            .bindings(&descriptor_bindings),
                        None,
                    )
                    .expect("Failed to create compute descriptor set layout");

                let descriptor_set = device
                    .allocate_descriptor_sets(
                        &vk::DescriptorSetAllocateInfo::builder()
                            .descriptor_pool(descriptor_pool)
                            .set_layouts(&[descriptor_set_layout]),
                    )
                    .expect("Failed to create compute descriptor set")[0];

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
                device.update_descriptor_sets(&[descriptor_set_write], &[]);

                (descriptor_set_layout, descriptor_set)
            };

            // compute pipeline
            let (pipeline_compute, pipeline_layout_compute) = {
                let render_code = read_spv(&mut Cursor::new(
                    &include_bytes!("../assets/shader_binaries/render.comp.spv")[..],
                ))
                .expect("Failed to read render compute shader spirv file");
                let render_shader_module = device
                    .create_shader_module(
                        &vk::ShaderModuleCreateInfo::builder().code(&render_code),
                        None,
                    )
                    .expect("Failed to create shader module");

                let pipeline_layout_compute = device
                    .create_pipeline_layout(
                        &vk::PipelineLayoutCreateInfo::builder()
                            .set_layouts(&[descriptor_set_layout_compute]),
                        None,
                    )
                    .expect("Failed to create pipeline layout");

                let pipeline_compute_ci = vk::ComputePipelineCreateInfo::builder()
                    .stage(vk::PipelineShaderStageCreateInfo {
                        stage: vk::ShaderStageFlags::COMPUTE,
                        module: render_shader_module,
                        p_name: CStr::from_bytes_with_nul_unchecked(b"main\0").as_ptr(),
                        ..Default::default()
                    })
                    .layout(pipeline_layout_compute);
                let pipeline_compute = device
                    .create_compute_pipelines(
                        vk::PipelineCache::null(),
                        &[pipeline_compute_ci.build()],
                        None,
                    )
                    .expect("failed to create compute pipeline")[0];

                // cleanup shader module
                device.destroy_shader_module(render_shader_module, None);

                (pipeline_compute, pipeline_layout_compute)
            };

            // calculate compute dispatch work group count
            let work_group_count = calc_work_group_count(surface_resolution.into());

            // render pass
            let renderpass = {
                let color_attachment_ref = [vk::AttachmentReference {
                    attachment: 0,
                    layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                }];
                let subpass_dependencies = [vk::SubpassDependency {
                    src_subpass: vk::SUBPASS_EXTERNAL,
                    src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_READ
                        | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                    dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    ..Default::default()
                }];
                let subpass_desc = vk::SubpassDescription::builder()
                    .color_attachments(&color_attachment_ref)
                    .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS);

                let renderpass_attachments = [vk::AttachmentDescription {
                    format: surface_format.format,
                    samples: vk::SampleCountFlags::TYPE_1,
                    load_op: vk::AttachmentLoadOp::CLEAR,
                    store_op: vk::AttachmentStoreOp::STORE,
                    final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
                    ..Default::default()
                }];
                let renderpass_ci = vk::RenderPassCreateInfo::builder()
                    .attachments(&renderpass_attachments)
                    .subpasses(std::slice::from_ref(&subpass_desc))
                    .dependencies(&subpass_dependencies);
                device
                    .create_render_pass(&renderpass_ci, None)
                    .expect("failed to create render pass")
            };

            // framebuffers
            let framebuffers: Vec<vk::Framebuffer> = swapchain_image_views
                .iter()
                .map(|&present_image_view| {
                    let framebuffer_attachments = [present_image_view];
                    let framebuffer_ci = vk::FramebufferCreateInfo::builder()
                        .render_pass(renderpass)
                        .attachments(&framebuffer_attachments)
                        .width(surface_resolution.width)
                        .height(surface_resolution.height)
                        .layers(1);
                    device
                        .create_framebuffer(&framebuffer_ci, None)
                        .expect("failed to create framebuffer")
                })
                .collect();

            // post processing descriptor set
            let (descriptor_set_layout_post, descriptor_set_post) = {
                let descriptor_bindings = [vk::DescriptorSetLayoutBinding::builder()
                    .binding(0)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                    .build()];
                let descriptor_set_layout = device
                    .create_descriptor_set_layout(
                        &vk::DescriptorSetLayoutCreateInfo::builder()
                            .bindings(&descriptor_bindings)
                            .build(),
                        None,
                    )
                    .expect("Failed to create post processing descriptor set layout");

                let descriptor_set = device
                    .allocate_descriptor_sets(
                        &vk::DescriptorSetAllocateInfo::builder()
                            .descriptor_pool(descriptor_pool)
                            .set_layouts(&[descriptor_set_layout])
                            .build(),
                    )
                    .expect("Failed to create post processing descriptor set")[0];

                let descriptor_set_write = vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set)
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&[vk::DescriptorImageInfo {
                        image_view: render_image_view,
                        image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                        sampler: render_sampler,
                    }])
                    .build();
                device.update_descriptor_sets(&[descriptor_set_write], &[]);

                (descriptor_set_layout, descriptor_set)
            };

            // post processing graphics pipeline
            let (pipeline_post, pipeline_layout_post) = {
                // shaders
                let vert_code = read_spv(&mut Cursor::new(
                    &include_bytes!("../assets/shader_binaries/post.vert.spv")[..],
                ))
                .expect("failed to read vertex shader spv file");
                let vert_shader_module = device
                    .create_shader_module(
                        &vk::ShaderModuleCreateInfo::builder().code(&vert_code),
                        None,
                    )
                    .expect("failed to create vertex shader module");

                let frag_code = read_spv(&mut Cursor::new(
                    &include_bytes!("../assets/shader_binaries/post.frag.spv")[..],
                ))
                .expect("failed to read fragment shader spv file");
                let frag_shader_module = device
                    .create_shader_module(
                        &vk::ShaderModuleCreateInfo::builder().code(&frag_code),
                        None,
                    )
                    .expect("failed to create fragment shader module");

                let shader_entry_name = CStr::from_bytes_with_nul_unchecked(b"main\0");
                let shader_stage_create_infos = [
                    vk::PipelineShaderStageCreateInfo {
                        module: vert_shader_module,
                        p_name: shader_entry_name.as_ptr(),
                        stage: vk::ShaderStageFlags::VERTEX,
                        ..Default::default()
                    },
                    vk::PipelineShaderStageCreateInfo {
                        s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                        module: frag_shader_module,
                        p_name: shader_entry_name.as_ptr(),
                        stage: vk::ShaderStageFlags::FRAGMENT,
                        ..Default::default()
                    },
                ];

                // pipeline layout
                let pipeline_layout_post = device
                    .create_pipeline_layout(
                        &vk::PipelineLayoutCreateInfo::builder()
                            .set_layouts(&[descriptor_set_layout_post]),
                        None,
                    )
                    .expect("failed to create post processing pipeline layout");

                // graphics pipeline create info
                let viewports = [vk::Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: surface_resolution.width as f32,
                    height: surface_resolution.height as f32,
                    min_depth: 0.0,
                    max_depth: 1.0,
                }];
                let scissors = [surface_resolution.into()];
                let viewport_state_info = vk::PipelineViewportStateCreateInfo::builder()
                    .scissors(&scissors)
                    .viewports(&viewports);

                let empty_vertex_input_state_info =
                    vk::PipelineVertexInputStateCreateInfo::builder();
                let vertex_input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo {
                    topology: vk::PrimitiveTopology::TRIANGLE_LIST,
                    ..Default::default()
                };
                let rasterization_info = vk::PipelineRasterizationStateCreateInfo {
                    front_face: vk::FrontFace::COUNTER_CLOCKWISE,
                    line_width: 1.0,
                    polygon_mode: vk::PolygonMode::FILL,
                    ..Default::default()
                };
                let multisample_state_info = vk::PipelineMultisampleStateCreateInfo {
                    rasterization_samples: vk::SampleCountFlags::TYPE_1,
                    ..Default::default()
                };
                let noop_stencil_state = vk::StencilOpState {
                    fail_op: vk::StencilOp::KEEP,
                    pass_op: vk::StencilOp::KEEP,
                    depth_fail_op: vk::StencilOp::KEEP,
                    compare_op: vk::CompareOp::ALWAYS,
                    ..Default::default()
                };
                let depth_state_info = vk::PipelineDepthStencilStateCreateInfo {
                    depth_test_enable: 1,
                    depth_write_enable: 1,
                    depth_compare_op: vk::CompareOp::LESS_OR_EQUAL,
                    front: noop_stencil_state,
                    back: noop_stencil_state,
                    max_depth_bounds: 1.0,
                    ..Default::default()
                };
                let color_blend_attachment_states = [vk::PipelineColorBlendAttachmentState {
                    blend_enable: 0,
                    src_color_blend_factor: vk::BlendFactor::SRC_COLOR,
                    dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_DST_COLOR,
                    color_blend_op: vk::BlendOp::ADD,
                    src_alpha_blend_factor: vk::BlendFactor::ZERO,
                    dst_alpha_blend_factor: vk::BlendFactor::ZERO,
                    alpha_blend_op: vk::BlendOp::ADD,
                    color_write_mask: vk::ColorComponentFlags::RGBA,
                }];
                let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
                    .logic_op(vk::LogicOp::CLEAR)
                    .attachments(&color_blend_attachment_states);
                let dynamic_state = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
                let dynamic_state_info =
                    vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_state);

                let pipeline_ci = vk::GraphicsPipelineCreateInfo::builder()
                    .stages(&shader_stage_create_infos)
                    .vertex_input_state(&empty_vertex_input_state_info)
                    .input_assembly_state(&vertex_input_assembly_state_info)
                    .viewport_state(&viewport_state_info)
                    .rasterization_state(&rasterization_info)
                    .multisample_state(&multisample_state_info)
                    .depth_stencil_state(&depth_state_info)
                    .color_blend_state(&color_blend_state)
                    .dynamic_state(&dynamic_state_info)
                    .layout(pipeline_layout_post)
                    .render_pass(renderpass)
                    .build();

                let pipeline_post = device
                    .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_ci], None)
                    .expect("failed to create post processing graphics pipeline")[0];

                // cleanup shader modules
                device.destroy_shader_module(vert_shader_module, None);
                device.destroy_shader_module(frag_shader_module, None);

                (pipeline_post, pipeline_layout_post)
            };

            // return self
            Renderer {
                entry,
                instance,
                device,
                surface_loader,
                queue,
                surface_resolution,
                work_group_count,
                swapchain_loader,
                swapchain,
                swapchain_image_views,
                command_pool,
                command_buffer_render,
                render_image,
                render_image_view,
                render_sampler,
                semaphore_present_complete,
                semaphore_rendering_complete,
                fence_render_commands_reuse,
                fence_setup_commands_reuse,
                surface,
                debug_call_back,
                debug_utils_loader,
                render_image_memory,
                descriptor_pool,
                descriptor_set_layout_compute,
                descriptor_set_layout_post,
                descriptor_set_compute,
                descriptor_set_post,
                pipeline_layout_compute,
                pipeline_compute,
                pipeline_layout_post,
                pipeline_post,
                renderpass,
                framebuffers,
            }
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();

            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);
            self.device
                .destroy_descriptor_set_layout(self.descriptor_set_layout_compute, None);
            self.device
                .destroy_descriptor_set_layout(self.descriptor_set_layout_post, None);
            self.device.destroy_command_pool(self.command_pool, None);

            self.device
                .destroy_pipeline_layout(self.pipeline_layout_compute, None);
            self.device
                .destroy_pipeline_layout(self.pipeline_layout_post, None);
            self.device.destroy_pipeline(self.pipeline_compute, None);
            self.device.destroy_pipeline(self.pipeline_post, None);

            for &framebuffer in self.framebuffers.iter() {
                self.device.destroy_framebuffer(framebuffer, None);
            }
            self.device.destroy_render_pass(self.renderpass, None);

            self.device
                .destroy_semaphore(self.semaphore_present_complete, None);
            self.device
                .destroy_semaphore(self.semaphore_rendering_complete, None);
            self.device
                .destroy_fence(self.fence_render_commands_reuse, None);
            self.device
                .destroy_fence(self.fence_setup_commands_reuse, None);

            self.device.free_memory(self.render_image_memory, None);
            self.device.destroy_image_view(self.render_image_view, None);
            self.device.destroy_image(self.render_image, None);
            self.device.destroy_sampler(self.render_sampler, None);

            for &image_view in self.swapchain_image_views.iter() {
                self.device.destroy_image_view(image_view, None);
            }
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
            self.surface_loader.destroy_surface(self.surface, None);

            self.device.destroy_device(None);

            self.debug_utils_loader
                .destroy_debug_utils_messenger(self.debug_call_back, None);
            self.instance.destroy_instance(None);
        }
    }
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

fn find_memorytype_index(
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
