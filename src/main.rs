// TODO:
// unwrap error handling (replace with expect)

use log::debug;
use log::error;
use log::info;
use log::warn;

use vulkano_win::VkSurfaceBuild;

use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::device::{Device, DeviceExtensions, Features};
use vulkano::format::Format;
use vulkano::image::attachment::AttachmentImage;
use vulkano::image::view::ImageView;
use vulkano::image::ImageUsage;
use vulkano::instance;
use vulkano::instance::debug::{DebugCallback, MessageSeverity, MessageType};
use vulkano::instance::{Instance, InstanceExtensions};
use vulkano::pipeline::{ComputePipeline, Pipeline, PipelineBindPoint};
use vulkano::swapchain::{self, AcquireError, Swapchain, SwapchainCreationError};
use vulkano::sync::{self, FlushError, GpuFuture};

use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

fn main() {
    // Initialize logger
    env_logger::init();

    // Create instance

    let instance_extensions = InstanceExtensions {
        ext_debug_utils: true,
        ..vulkano_win::required_extensions()
    };

    let layers = vec!["VK_LAYER_KHRONOS_validation"];
    let mut available_layers = instance::layers_list().unwrap();
    while let Some(l) = available_layers.next() {
        println!("\t{}", l.name());
    }
    assert!(
        layers
            .iter()
            .all(|&l| instance::layers_list().unwrap().any(|la| la.name() == l)),
        "requested Vulkan layer(s) not available"
    );

    let instance = Instance::new(None, vulkano::Version::V1_2, &instance_extensions, layers)
        .expect("failed to create Vulkan instance");

    // Vulkan debug callback

    let message_severity = MessageSeverity {
        verbose: false,
        ..MessageSeverity::all()
    };
    let message_type = MessageType::all();
    let _debug_callback = DebugCallback::new(&instance, message_severity, message_type, |msg| {
        let ty = if msg.ty.general {
            "general"
        } else if msg.ty.validation {
            "validation"
        } else if msg.ty.performance {
            "performance"
        } else {
            panic!("no-impl");
        };

        if msg.severity.error {
            error!(
                "Vulkan - {} {}: {}",
                msg.layer_prefix.unwrap_or("unknown"),
                ty,
                msg.description
            );
        } else if msg.severity.warning {
            warn!(
                "Vulkan - {} {}: {}",
                msg.layer_prefix.unwrap_or("unknown"),
                ty,
                msg.description
            );
        } else if msg.severity.information {
            info!(
                "Vulkan - {} {}: {}",
                msg.layer_prefix.unwrap_or("unknown"),
                ty,
                msg.description
            );
        } else if msg.severity.verbose {
            debug!(
                "Vulkan - {} {}: {}",
                msg.layer_prefix.unwrap_or("unknown"),
                ty,
                msg.description
            );
        } else {
            panic!("no-impl");
        };
    })
    .ok();

    // Create winit window
    let event_loop = EventLoop::new();
    let surface = WindowBuilder::new()
        .build_vk_surface(&event_loop, instance.clone())
        .unwrap();

    // Device extensions
    let device_extensions = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::none()
    };

    // Choose a physical device and queue family
    let (physical_device, queue_family) = PhysicalDevice::enumerate(&instance)
        // Essential Requirements:
        // Filter by extension support.
        .filter(|&p| p.supported_extensions().is_superset_of(&device_extensions))
        // Filter by queue functionality and then map to tuple(s) with chosen queue family.
        .filter_map(|p| {
            p.queue_families()
                // Check for a queue family that supports graphics and can present to our surface.
                .find(|&q| q.supports_graphics() && surface.is_supported(q).unwrap_or(false))
                // Return a tuple containing the physical device and queue family.
                // If no queue was found then None is returned and this device is filtered out.
                .map(|q| (p, q))
        })
        // OPTIMAL REQUIREMENTS:
        // Chose from the filtered list based on the best score of desired properties.
        .min_by_key(|(p, _)| {
            // Return a better (lower) score depending on the device type (we ideally want a
            // discrete GPU).
            match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                PhysicalDeviceType::Other => 4,
            }
        })
        .expect("no device available");

    info!(
        "Using device: {} (type: {:?})",
        physical_device.properties().device_name,
        physical_device.properties().device_type,
    );

    // Create device and queues.
    let (device, mut queues) = Device::new(
        physical_device,
        &Features::none(),
        &physical_device
            .required_extensions()
            .union(&device_extensions),
        [(queue_family, 0.5)].iter().cloned(),
    )
    .expect("failed to create device");

    // Single out the first queue for use
    let queue = queues.next().unwrap();

    // Create swapchain and swapchain images
    let mut dimensions: [u32; 2] = surface.window().inner_size().into();
    let (mut swapchain, swapchain_images) = {
        // Surface capabilities
        let caps = surface.capabilities(physical_device).unwrap();

        // The alpha mode indicates how the alpha value of the final image will behave. For example,
        // you can choose whether the window will be opaque or transparent.
        let composite_alpha = caps.supported_composite_alpha.iter().next().unwrap();

        // Swapchain image format
        let format = caps.supported_formats[0].0;

        Swapchain::start(device.clone(), surface.clone())
            .num_images(caps.min_image_count)
            .format(format)
            .dimensions(dimensions)
            .usage(ImageUsage {
                color_attachment: true,
                transfer_destination: true,
                ..ImageUsage::none()
            })
            .sharing_mode(&queue)
            .composite_alpha(composite_alpha)
            .build()
            .unwrap()
    };

    // Compute pipeline

    mod cs {
        vulkano_shaders::shader! {
            ty: "compute",
            path: "assets/shaders/render.comp"
        }
    }

    let pipeline = ComputePipeline::new(
        device.clone(),
        cs::load(device.clone())
            .expect("failed to create compute shader module")
            .entry_point("main")
            .expect("failed to specify compute shader entry point"),
        &(),
        None,
        |_| {},
    )
    .expect("failed to create compute pipeline");

    // Render images

    let render_images = vec![
        AttachmentImage::with_usage(
            device.clone(),
            dimensions,
            Format::R8G8B8A8_UNORM,
            ImageUsage {
                storage: true,
                transfer_source: true,
                ..ImageUsage::none()
            }
        )
        .expect("failed to create a render image");
        swapchain_images.len()
    ];

    // Descriptor sets

    let desc_layout = pipeline.layout().descriptor_set_layouts().get(0).unwrap();
    let desc_sets = render_images
        .iter()
        .map(|image| {
            PersistentDescriptorSet::new(
                desc_layout.clone(),
                [WriteDescriptorSet::image_view(
                    0,
                    ImageView::new(image.clone()).unwrap(),
                )],
            )
            .unwrap()
        })
        .collect::<Vec<_>>();

    let mut recreate_swapchain = false;

    let mut previous_frame_end = Some(sync::now(device.clone()).boxed());

    // window loop
    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                recreate_swapchain = true;
            }
            Event::RedrawEventsCleared => {
                // It is important to call this function from time to time, otherwise resources will keep
                // accumulating and you will eventually reach an out of memory error.
                // Calling this function polls various fences in order to determine what the GPU has
                // already processed, and frees the resources that are no longer needed.
                previous_frame_end.as_mut().unwrap().cleanup_finished();

                // Recreate swapchain (e.g. when window resized)
                if recreate_swapchain {
                    dimensions = surface.window().inner_size().into();
                    let (new_swapchain, _new_images) =
                        match swapchain.recreate().dimensions(dimensions).build() {
                            Ok(r) => r,
                            // This error tends to happen when the user is manually resizing the window.
                            // Simply restarting the loop is the easiest way to fix this issue.
                            Err(SwapchainCreationError::UnsupportedDimensions) => return,
                            Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
                        };

                    swapchain = new_swapchain;
                    recreate_swapchain = false;
                }

                // Aquire swapchain image
                let (image_num, suboptimal, acquire_future) =
                    match swapchain::acquire_next_image(swapchain.clone(), None) {
                        Ok(r) => r,
                        Err(AcquireError::OutOfDate) => {
                            recreate_swapchain = true;
                            return;
                        }
                        Err(e) => panic!("Failed to acquire next image: {:?}", e),
                    };

                // acquire_next_image can be successful, but suboptimal. This means that the swapchain image
                // will still work, but it may not display correctly. With some drivers this can be when
                // the window resizes, but it may not cause the swapchain to become out of date.
                if suboptimal {
                    recreate_swapchain = true;
                }

                // Command buffer builder
                // Building a command buffer is an expensive operation (usually a few hundred
                // microseconds), but it is known to be a hot path in the driver and is expected to be
                // optimized.
                let mut builder = AutoCommandBufferBuilder::primary(
                    device.clone(),
                    queue.family(),
                    CommandBufferUsage::OneTimeSubmit,
                )
                .unwrap();

                // Record commands
                let push_constants = cs::ty::PushConstants {
                    image_size: dimensions,
                };
                builder
                    .bind_pipeline_compute(pipeline.clone())
                    .bind_descriptor_sets(
                        PipelineBindPoint::Compute,
                        pipeline.layout().clone(),
                        0,
                        desc_sets[image_num].clone(),
                    )
                    .push_constants(pipeline.layout().clone(), 0, push_constants)
                    .dispatch([dimensions[0] / 8, dimensions[1] / 8, 1])
                    .expect("failed to record compute dispatch")
                    .blit_image(
                        render_images[image_num].clone(),
                        [0, 0, 0],
                        [dimensions[0] as i32, dimensions[1] as i32, 0],
                        0,
                        0,
                        swapchain_images[image_num].clone(),
                        [0, 0, 0],
                        [dimensions[0] as i32, dimensions[1] as i32, 0],
                        0,
                        0,
                        1,
                        vulkano::sampler::Filter::Nearest,
                    )
                    .expect("failed to record render image blit");

                // Create command buffer
                let command_buffer = builder
                    .build()
                    .expect("failed to build primary command buffer");

                let future = previous_frame_end
                    .take()
                    .unwrap()
                    .join(acquire_future)
                    .then_execute(queue.clone(), command_buffer)
                    .unwrap()
                    .then_swapchain_present(queue.clone(), swapchain.clone(), image_num)
                    .then_signal_fence_and_flush();

                match future {
                    Ok(future) => {
                        previous_frame_end = Some(future.boxed());
                    }
                    Err(FlushError::OutOfDate) => {
                        recreate_swapchain = true;
                        previous_frame_end = Some(sync::now(device.clone()).boxed());
                    }
                    Err(e) => {
                        println!("Failed to flush future: {:?}", e);
                        previous_frame_end = Some(sync::now(device.clone()).boxed());
                    }
                }
            }
            _ => (),
        }
    });
}
