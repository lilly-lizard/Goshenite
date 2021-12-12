//! # About
//!
//! Some comments explaining Vulkan code are adapted from the (Vulkano)[https://github.com/vulkano-rs/vulkano]
//! triangle.rs example and may be helpful for people unfamiliar with Vulkan. This application uses Vulkan
//! 1.2 and if you are interested in finer details about Vulkan functionality or function arguments I
//! recommend referencing and getting familiar with the (Vulkan specification)[https://www.khronos.org/registry/vulkan/specs/1.2-extensions/html/].
//!
//! Quick find comments:
//! - LOG: log output
//! - TESTING: temporary testing (to be deleted before commiting)

// TODO:
// validation layers
// unwrap error handling

use std::sync::Arc;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer, TypedBufferAccess};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, SubpassContents};
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::device::{Device, DeviceExtensions, Features};
use vulkano::image::view::ImageView;
use vulkano::image::{ImageAccess, ImageUsage, SwapchainImage};
use vulkano::instance::Instance;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::GraphicsPipeline;
use vulkano::render_pass::{Framebuffer, RenderPass, Subpass};
use vulkano::swapchain::{self, AcquireError, Swapchain, SwapchainCreationError};
use vulkano::sync::{self, FlushError, GpuFuture};
use vulkano::Version;
use vulkano_win::VkSurfaceBuild;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

fn main() {
    // The first step of any Vulkan program is to create an instance.
    // When we create an instance, we have to pass a list of extensions that we want to enable.
    // All the window-drawing functionalities are part of non-core extensions that we need
    // to enable manually. To do so, we ask the `vulkano_win` crate for the list of extensions
    // required to draw to a window.
    let instance_extensions = vulkano_win::required_extensions();
    let instance = Instance::new(None, vulkano::Version::V1_2, &instance_extensions, None).unwrap();

    // Create a window with `winit::EventLoop` and `winit::WindowBuilder`, then create a
    // `vulkano::swapchain::Surface` object by calling the `VkSurfaceBuild::build_vk_surface` method
    // provided by `VkSurfaceBuild` (Note the `VkSurfaceBuild` trait must be imported).
    // The `vulkano::swapchain::Surface` object contains both a cross-platform winit window and a
    // cross-platform Vulkan surface that represents the surface of the window.
    let event_loop = EventLoop::new();
    let surface = WindowBuilder::new()
        .build_vk_surface(&event_loop, instance.clone())
        .unwrap();

    // Device extensions:
    // In order to present images to a surface, we need a `Swapchain`, which is provided by the
    // `khr_swapchain` extension.
    let device_extensions = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::none()
    };

    // Choose a physical device and queue family:
    // Enumerate the physical devices supporting Vulkan then narrow down the options based on the
    // application requirements.
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
        .unwrap();

    // LOG
    println!(
        "Using device: {} (type: {:?})",
        physical_device.properties().device_name,
        physical_device.properties().device_type,
    );

    // Initialize the Vulkan (logical) device and create queues for submitting commands.
    let (device, mut queues) = Device::new(
        physical_device,
        &Features::none(),
        &physical_device
            .required_extensions()
            .union(&device_extensions),
        [(queue_family, 0.5)].iter().cloned(),
    )
    .unwrap();

    // We'll use a single queue in this application.
    let queue = queues.next().unwrap();

    // Before we can draw on the surface, we have to create what is called a swapchain. Creating
    // a swapchain allocates the buffers that will contain the image that will ultimately be
    // presented to the screen.
    let (mut swapchain, swapchain_images) = {
        // Querying the capabilities of the surface. When we create the swapchain we can only
        // pass values that are allowed by the capabilities.
        let caps = surface.capabilities(physical_device).unwrap();

        // The alpha mode indicates how the alpha value of the final image will behave. For example,
        // you can choose whether the window will be opaque or transparent.
        let composite_alpha = caps.supported_composite_alpha.iter().next().unwrap();

        // Choose the internal format that the images will have.
        let format = caps.supported_formats[0].0;

        // The dimensions of the window, only used to initially setup the swapchain.
        // NOTE:
        // On some drivers the swapchain dimensions are specified by `caps.current_extent` and the
        // swapchain size must use these dimensions.
        // These dimensions are always the same as the window dimensions.
        //
        // However, other drivers don't specify a value, i.e. `caps.current_extent` is `None`
        // These drivers will allow anything, but the only sensible value is the window dimensions.
        //
        // Both of these cases need the swapchain to use the window dimensions, so we just use that.
        let dimensions: [u32; 2] = surface.window().inner_size().into();

        Swapchain::start(device.clone(), surface.clone())
            .num_images(caps.min_image_count)
            .format(format)
            .dimensions(dimensions)
            .usage(ImageUsage::color_attachment())
            .sharing_mode(&queue)
            .composite_alpha(composite_alpha)
            .build()
            .unwrap()
    };

    // We now create a buffer that will store the shape of our triangle.
    // We use #[repr(C)] here to force rustc to not do anything funky with our data, although for this
    // particular example, it doesn't actually change the in-memory representation.
    #[repr(C)]
    #[derive(Default, Debug, Clone)]
    struct Vertex {
        position: [f32; 2],
    }
    vulkano::impl_vertex!(Vertex, position);

    // TESTING
    struct VertexTest {
        position: glam::Vec3,
    }
    vulkano::impl_vertex!(VertexTest, position);

    let vertex_buffer = CpuAccessibleBuffer::from_iter(
        device.clone(),
        BufferUsage::all(),
        false,
        [
            Vertex {
                position: [-0.5, -0.25],
            },
            Vertex {
                position: [0.0, 0.5],
            },
            Vertex {
                position: [0.25, -0.1],
            },
        ]
        .iter()
        .cloned(),
    )
    .unwrap();

    // Create vertex and fragment shaders using the `vulkano_shaders::shader!` macro.
    // You can pass either `src` for GLSL source, `path` for a path to a SPIR-V file (relative to
    // Cargo.toml). See the (docs)[https://docs.rs/vulkano-shaders/] for more info.
    mod vs {
        vulkano_shaders::shader! {
            ty: "vertex",
            src: "
				#version 450

				layout(location = 0) in vec2 position;

				void main() {
					gl_Position = vec4(position, 0.0, 1.0);
				}
			"
        }
    }
    mod fs {
        vulkano_shaders::shader! {
            ty: "fragment",
            src: "
				#version 450

				layout(location = 0) out vec4 f_color;

				void main() {
					f_color = vec4(1.0, 0.0, 0.0, 1.0);
				}
			"
        }
    }

    let vs = vs::load(device.clone()).unwrap();
    let fs = fs::load(device.clone()).unwrap();

    // The next step is to create a *render pass*, which is an object that describes where the
    // output of the graphics pipeline will go. It describes the layout of the images
    // where the colors, depth and/or stencil information will be written.
    // For more info on the parameters defining a render pass see https://www.khronos.org/registry/vulkan/specs/1.2-extensions/html/chap8.html#VkRenderPassCreateInfo
    let render_pass = vulkano::single_pass_renderpass!(
        device.clone(),
        attachments: {
            // `color` is a custom name we give to the first and only attachment.
            color: {
                load: Clear,
                store: Store,
                format: swapchain.format(),
                samples: 1,
            }
        },
        pass: {
            // We use the attachment named `color` as the one and only color attachment.
            color: [color],
            // No depth-stencil attachment is indicated with empty brackets.
            depth_stencil: {}
        }
    )
    .unwrap();

    // A Vulkan pipeline describes the stages and settings of how data proceeds through the GPU.
    // See the *Pipelines* section of the Vulkan spec for more info (it has a cool block diagram).
    let pipeline = GraphicsPipeline::start()
        // Describe the vertex data layout
        .vertex_input_state(BuffersDefinition::new().vertex::<Vertex>())
        // Settings for the input assembly pipeline stage
        .input_assembly_state(InputAssemblyState::new())
        // Use a shader for the programmable vertex shading stage
        .vertex_shader(vs.entry_point("main").unwrap(), ())
        // Viewport settings for rasterization
        .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
        // Use a shader for the programmable fragment shading stage
        .fragment_shader(fs.entry_point("main").unwrap(), ())
        // Set the render pass and subpass that this pipeline will be used in
        .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
        // Call vkCreatePipeline
        .build(device.clone())
        .unwrap();

    // We set the pipeline viewport settings to dynamic meaning we won't have to recreate the
    // pipeline when the window is resized
    let mut viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [0.0, 0.0],
        depth_range: 0.0..1.0,
    };

    // Framebuffers are the Vulkan objects that the graphics pipeline renders to, meaning the
    // images need to match the layout described by the render pass and fragment shader code
    let mut framebuffers =
        window_size_dependent_setup(&swapchain_images, render_pass.clone(), &mut viewport);
}

/// This method is called once during initialization, then again whenever the window is resized
fn window_size_dependent_setup(
    swapchain_images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<RenderPass>,
    viewport: &mut Viewport,
) -> Vec<Arc<Framebuffer>> {
    let dimensions = swapchain_images[0].dimensions().width_height();
    viewport.dimensions = [dimensions[0] as f32, dimensions[1] as f32];

    swapchain_images
        .iter()
        .map(|image| {
            let view = ImageView::new(image.clone()).unwrap();
            Framebuffer::start(render_pass.clone())
                .add(view)
                .unwrap()
                .build()
                .unwrap()
        })
        .collect::<Vec<_>>()
}
