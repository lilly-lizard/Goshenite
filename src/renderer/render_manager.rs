use super::{
    geometry_pass::GeometryPass, gui_renderer::GuiRenderer, lighting_pass::LightingPass,
    shader_interfaces::push_constants::CameraPushConstants, vulkan_helper::*,
};
use crate::{
    engine::object::{object_collection::ObjectCollection, objects_delta::ObjectsDelta},
    renderer::renderer_config::{
        ENABLE_VULKAN_VALIDATION, G_BUFFER_FORMAT_NORMAL, G_BUFFER_FORMAT_PRIMITIVE_ID,
    },
    user_interface::{camera::Camera, gui::Gui},
};
use anyhow::{anyhow, Context};
use egui::TexturesDelta;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::sync::Arc;
use vulkano::{
    command_buffer::{
        self,
        allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo},
        RenderPassBeginInfo, SubpassContents,
    },
    descriptor_set::allocator::StandardDescriptorSetAllocator,
    device::{Device, DeviceCreateInfo, Queue, QueueCreateInfo},
    format::ClearValue,
    image::{view::ImageView, AttachmentImage, ImageAccess, SampleCount, SwapchainImage},
    instance::debug::DebugUtilsMessenger,
    instance::{Instance, InstanceCreateInfo},
    memory::allocator::StandardMemoryAllocator,
    pipeline::graphics::viewport::Viewport,
    render_pass::{Framebuffer, RenderPass, Subpass},
    swapchain::{self, Surface, Swapchain, SwapchainCreationError, SwapchainPresentInfo},
    sync::{self, FlushError, GpuFuture},
    VulkanLibrary,
};
use winit::window::Window;

// number of primary and secondary command buffers to initially allocate
const PRE_ALLOCATE_PRIMARY_COMMAND_BUFFERS: usize = 64;
const PRE_ALLOCATE_SECONDARY_COMMAND_BUFFERS: usize = 0;

/// Contains Vulkan resources and methods to manage rendering
pub struct RenderManager {
    device: Arc<Device>,
    render_queue: Arc<Queue>,
    _transfer_queue: Arc<Queue>,
    _debug_callback: Option<DebugUtilsMessenger>,

    window: Arc<Window>,
    _surface: Arc<Surface>,
    swapchain: Arc<Swapchain>,
    swapchain_image_views: Vec<Arc<ImageView<SwapchainImage>>>,
    is_srgb_framebuffer: bool,

    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer_allocator: StandardCommandBufferAllocator,
    descriptor_allocator: Arc<StandardDescriptorSetAllocator>,

    viewport: Viewport,
    g_buffer_normal: Arc<ImageView<AttachmentImage>>,
    g_buffer_primitive_id: Arc<ImageView<AttachmentImage>>,
    render_pass: Arc<RenderPass>,
    framebuffers: Vec<Arc<Framebuffer>>,
    clear_values: [Option<ClearValue>; 3],

    geometry_pass: GeometryPass,
    lighting_pass: LightingPass,
    //overlay_pass: OverlayPass,
    gui_pass: GuiRenderer,
    /// Can be used to synchronize commands with the submission for the previous frame
    future_previous_frame: Option<Box<dyn GpuFuture>>,
    /// Indicates that the swapchain needs to be recreated next frame
    recreate_swapchain: bool,
}

// Public functions

impl RenderManager {
    /// Initializes Vulkan resources. If renderer fails to initialize, returns a string explanation.
    pub fn new(window: Arc<Window>, object_collection: &ObjectCollection) -> anyhow::Result<Self> {
        let vulkan_library = VulkanLibrary::new().context("loading vulkan library")?;
        info!(
            "loaded vulkan library, api version = {}",
            vulkan_library.api_version()
        );

        // required instance extensions for platform surface rendering
        let mut instance_extensions = vulkano_win::required_extensions(&vulkan_library);
        let mut instance_layers: Vec<String> = Vec::new();

        // check for validation layer/debug callback support
        let enable_debug_callback = if ENABLE_VULKAN_VALIDATION {
            if add_debug_validation(
                vulkan_library.clone(),
                &mut instance_extensions,
                &mut instance_layers,
            )
            .is_ok()
            {
                info!("enabling Vulkan validation layers and debug callback");
                true
            } else {
                warn!("validation layer debug callback requested but cannot be enabled");
                false
            }
        } else {
            debug!("Vulkan validation layers disabled via config.rs");
            false
        };

        // create instance
        debug!("enabling instance extensions: {:?}", instance_extensions);
        debug!("enabling vulkan layers: {:?}", instance_layers);
        let instance = Instance::new(
            vulkan_library.clone(),
            InstanceCreateInfo {
                enabled_extensions: instance_extensions,
                enumerate_portability: true, // enable enumerating devices that use non-conformant vulkan implementations. (ex. MoltenVK)
                enabled_layers: instance_layers,
                ..Default::default()
            },
        )
        .context("creating vulkan instance")?;

        // setup debug callback
        let debug_callback = if enable_debug_callback {
            setup_debug_callback(instance.clone())
        } else {
            None
        };

        // create surface
        let surface = vulkano_win::create_surface_from_winit(window.clone(), instance.clone())
            .context("creating vulkan surface")?;

        // required device extensions
        let device_extensions = required_device_extensions();
        debug!("required vulkan device extensions: {:?}", device_extensions);

        // print available physical devices
        debug!("available Vulkan physical devices:");
        for pd in instance
            .enumerate_physical_devices()
            .context("enumerating physical devices")?
        {
            debug!("\t{}", pd.properties().device_name);
        }
        // choose physical device and queue families
        let ChoosePhysicalDeviceReturn {
            physical_device,
            render_queue_family,
            transfer_queue_family,
        } = choose_physical_device(instance.clone(), &device_extensions, &surface)?;
        info!(
            "Using Vulkan device: {} (type: {:?})",
            physical_device.properties().device_name,
            physical_device.properties().device_type,
        );
        debug!("render queue family index = {}", render_queue_family);
        debug!("transfer queue family index = {}", transfer_queue_family);

        // queue create info(s) for creating render and transfer queues
        let single_queue = (render_queue_family == transfer_queue_family)
            && (physical_device.queue_family_properties()[render_queue_family as usize]
                .queue_count
                == 1);
        let queue_create_infos = if render_queue_family == transfer_queue_family {
            vec![QueueCreateInfo {
                queue_family_index: render_queue_family,
                queues: if single_queue {
                    vec![0.5]
                } else {
                    vec![0.5; 2]
                },
                ..Default::default()
            }]
        } else {
            vec![
                QueueCreateInfo {
                    queue_family_index: render_queue_family,
                    ..Default::default()
                },
                QueueCreateInfo {
                    queue_family_index: transfer_queue_family,
                    ..Default::default()
                },
            ]
        };

        // create device and queues
        let (device, mut queues) = Device::new(
            physical_device.clone(),
            DeviceCreateInfo {
                enabled_extensions: device_extensions,
                enabled_features: required_features(),
                queue_create_infos,
                ..Default::default()
            },
        )
        .context("creating vulkan device and queues")?;
        let render_queue = queues
            .next()
            .expect("requested 1 queue from render_queue_family");
        let transfer_queue = if single_queue {
            render_queue.clone()
        } else {
            queues.next().expect("requested 1 unique transfer queue")
        };

        // swapchain
        let (swapchain, swapchain_images) = create_swapchain(
            device.clone(),
            physical_device.clone(),
            surface.clone(),
            &window,
        )?;
        debug!(
            "initial swapchain image size = {:?}",
            swapchain_images[0].dimensions()
        );
        let is_srgb_framebuffer = is_srgb_framebuffer(swapchain_images[0].clone());
        let swapchain_image_views = swapchain_images
            .iter()
            .map(|image| ImageView::new_default(image.clone()))
            .collect::<Result<Vec<_>, _>>()
            .context("creating swapchain image views")?;

        // dynamic viewport
        let viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [
                swapchain_images[0].dimensions().width() as f32,
                swapchain_images[0].dimensions().height() as f32,
            ],
            depth_range: 0.0..1.0,
        };

        // create render_pass
        let multisample = SampleCount::Sample1;
        let render_pass = create_render_pass(device.clone(), &swapchain_image_views, multisample)?;

        // describe subpasses
        let subpass_gbuffer = Subpass::from(
            render_pass.clone(),
            render_pass_indices::SUBPASS_GBUFFER as u32,
        )
        .ok_or(anyhow!(
            "render pass does not contain subpass at index {}",
            render_pass_indices::SUBPASS_GBUFFER
        ))?;
        let subpass_swapchain = Subpass::from(
            render_pass.clone(),
            render_pass_indices::SUBPASS_SWAPCHAIN as u32,
        )
        .ok_or(anyhow!(
            "render pass does not contain subpass at index {}",
            render_pass_indices::SUBPASS_SWAPCHAIN
        ))?;

        // allocators
        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
        let command_buffer_allocator = StandardCommandBufferAllocator::new(
            device.clone(),
            StandardCommandBufferAllocatorCreateInfo {
                primary_buffer_count: PRE_ALLOCATE_PRIMARY_COMMAND_BUFFERS,
                secondary_buffer_count: PRE_ALLOCATE_SECONDARY_COMMAND_BUFFERS,
                ..StandardCommandBufferAllocatorCreateInfo::default()
            },
        );
        let descriptor_allocator = Arc::new(StandardDescriptorSetAllocator::new(device.clone()));

        // g-buffers
        let g_buffer_normal = create_g_buffer(
            &memory_allocator,
            swapchain_images[0].dimensions().width_height(),
            G_BUFFER_FORMAT_NORMAL,
        )?;
        let g_buffer_primitive_id = create_g_buffer(
            &memory_allocator,
            swapchain_images[0].dimensions().width_height(),
            G_BUFFER_FORMAT_PRIMITIVE_ID,
        )?;

        // create framebuffers
        let framebuffers = create_framebuffers(
            render_pass.clone(),
            &swapchain_image_views,
            g_buffer_normal.clone(),
            g_buffer_primitive_id.clone(),
        )?;
        let mut clear_values: [Option<ClearValue>; 3] = Default::default();
        clear_values[render_pass_indices::ATTACHMENT_SWAPCHAIN] = Some([0.0, 0.0, 0.0, 1.0].into());
        clear_values[render_pass_indices::ATTACHMENT_NORMAL] = Some([0.0; 4].into());
        clear_values[render_pass_indices::ATTACHMENT_PRIMITIVE_ID] = Some([0u32; 4].into());

        // init lighting pass
        let lighting_pass = LightingPass::new(
            device.clone(),
            &descriptor_allocator,
            g_buffer_normal.clone(),
            g_buffer_primitive_id.clone(),
            subpass_swapchain.clone(),
        )?;

        // init geometry pass
        let geometry_pass = GeometryPass::new(
            device.clone(),
            memory_allocator.clone(),
            descriptor_allocator.clone(),
            subpass_gbuffer,
            object_collection,
        )?;

        // init overlay pass
        //let overlay_pass =
        //    OverlayPass::new(device.clone(), &memory_allocator, subpass_swapchain.clone())?;

        // init gui renderer
        let gui_pass = GuiRenderer::new(
            device.clone(),
            memory_allocator.clone(),
            render_queue.clone(),
            subpass_swapchain.clone(),
        )?;

        // create futures used for frame synchronization
        let future_previous_frame = Some(sync::now(device.clone()).boxed());

        Ok(RenderManager {
            device,
            render_queue,
            _transfer_queue: transfer_queue,
            _debug_callback: debug_callback,

            window,
            _surface: surface,
            swapchain,
            swapchain_image_views,
            is_srgb_framebuffer,

            memory_allocator,
            command_buffer_allocator,
            descriptor_allocator,

            viewport,
            g_buffer_normal,
            g_buffer_primitive_id,
            render_pass,
            framebuffers,
            clear_values,

            geometry_pass,
            lighting_pass,
            //overlay_pass,
            gui_pass,
            future_previous_frame,
            recreate_swapchain: false,
        })
    }

    pub fn update_object_buffers(
        &mut self,
        object_collection: &ObjectCollection,
        object_delta: ObjectsDelta,
    ) -> anyhow::Result<()> {
        self.geometry_pass
            .update_object_buffers(object_collection, object_delta)
    }

    pub fn update_gui_textures(
        &mut self,
        textures_delta_vec: Vec<TexturesDelta>,
    ) -> anyhow::Result<()> {
        self.wait_and_unlock_previous_frame();

        self.future_previous_frame = Some(
            self.gui_pass.update_textures(
                self.future_previous_frame
                    .take()
                    .unwrap_or(sync::now(self.device.clone()).boxed()), // should never be None anyway...
                &self.command_buffer_allocator,
                &self.descriptor_allocator,
                textures_delta_vec,
                self.render_queue.clone(),
            )?,
        );
        Ok(())
    }

    /// Submits Vulkan commands for rendering a frame.
    pub fn render_frame(
        &mut self,
        window_resize: bool,
        gui: &mut Gui,
        camera: &mut Camera,
    ) -> anyhow::Result<()> {
        self.wait_and_unlock_previous_frame();

        self.recreate_swapchain = self.recreate_swapchain || window_resize;
        if self.recreate_swapchain {
            // recreate swapchain
            self.recreate_swapchain()?;
        }

        // blocks when no images currently available (all have been submitted already)
        let (swapchain_index, suboptimal, acquire_future) =
            match swapchain::acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => r,
                Err(swapchain::AcquireError::OutOfDate) => {
                    // recreate swapchain and skip frame render
                    return self.recreate_swapchain();
                }
                Err(e) => {
                    return Err(anyhow!(e)).context("aquiring swapchain image");
                }
            };
        // 'suboptimal' indicates that the swapchain image will still work but may not be displayed correctly
        // we'll render the frame anyway hehe
        if suboptimal {
            debug!(
                "suboptimal swapchain image {}, rendering anyway...",
                swapchain_index
            );
            self.recreate_swapchain = true;
        }

        // todo shouldn't need to recreate each frame?
        //self.geometry_pass
        //    .update_buffers(&self.descriptor_allocator)?;

        // camera data used in geometry and lighting passes
        let camera_push_constants = CameraPushConstants::new(
            glam::DMat4::inverse(&(camera.proj_matrix() * camera.view_matrix())).as_mat4(),
            camera.position().as_vec3(),
            self.viewport.dimensions,
        );

        // record command buffer
        let mut builder = command_buffer::AutoCommandBufferBuilder::primary(
            &self.command_buffer_allocator,
            self.render_queue.queue_family_index(),
            command_buffer::CommandBufferUsage::OneTimeSubmit,
        )
        .context("beginning primary command buffer")?;

        // begin render pass
        builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: self.clear_values.into(),
                    ..RenderPassBeginInfo::framebuffer(
                        self.framebuffers[swapchain_index as usize].clone(),
                    )
                },
                SubpassContents::Inline,
            )
            .context("recording begin render pass command")?;

        // geometry ray-marching pass (outputs to g-buffer)
        self.geometry_pass.record_commands(
            &mut builder,
            camera_push_constants,
            self.viewport.clone(),
        )?;

        builder
            .next_subpass(SubpassContents::Inline)
            .context("recording next subpass command")?;

        // execute deferred lighting pass (reads from g-buffer)
        self.lighting_pass.record_commands(
            &mut builder,
            camera_push_constants,
            self.viewport.clone(),
        )?;

        // draw editor overlay
        //self.overlay_pass
        //    .record_commands(&mut builder, camera, self.viewport.clone())?;

        // render gui
        self.gui_pass.record_commands(
            &mut builder,
            gui,
            self.is_srgb_framebuffer,
            self.viewport.dimensions,
        )?;

        // end render pass
        builder
            .end_render_pass()
            .context("recording end render pass command")?;
        let command_buffer = builder.build().context("building frame command buffer")?;

        // submit
        let future = self
            .future_previous_frame
            .take()
            .unwrap_or(sync::now(self.device.clone()).boxed()) // should never be None anyway...
            .join(acquire_future)
            .then_execute(self.render_queue.clone(), command_buffer)
            .context("executing vulkan primary command buffer")?
            .then_swapchain_present(
                self.render_queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(
                    self.swapchain.clone(),
                    swapchain_index,
                ),
            )
            .then_signal_fence_and_flush();

        match future {
            Ok(future) => {
                self.future_previous_frame = Some(future.boxed());
            }
            Err(FlushError::OutOfDate) => {
                self.recreate_swapchain = true;
                self.future_previous_frame = Some(sync::now(self.device.clone()).boxed());
            }
            Err(e) => {
                error!("Failed to flush future: {}", e);
                self.future_previous_frame = Some(sync::now(self.device.clone()).boxed());
            }
        }
        Ok(())
    }
}

// Private functions

impl RenderManager {
    /// Recreates the swapchain, g-buffers and assiciated descriptor sets, then unsets `recreate_swapchain` trigger.
    fn recreate_swapchain(&mut self) -> anyhow::Result<()> {
        let new_size: [u32; 2] = self.window.inner_size().into();

        debug!(
            "recreating swapchain and render targets to size: {:?}",
            new_size
        );
        let (new_swapchain, swapchain_images) =
            match self.swapchain.recreate(swapchain::SwapchainCreateInfo {
                image_extent: new_size,
                ..self.swapchain.create_info()
            }) {
                Ok(r) => r,
                // this error tends to happen when the user is manually resizing the window.
                // simply restarting the loop is the easiest way to fix this issue.
                Err(e @ SwapchainCreationError::ImageExtentNotSupported { .. }) => {
                    debug!("failed to recreate swapchain due to {}", e);
                    return Ok(());
                }
                Err(e) => return Err(e).context("recreating swapchain"),
            };

        self.swapchain = new_swapchain;
        self.swapchain_image_views = swapchain_images
            .iter()
            .map(|image| {
                ImageView::new_default(image.clone()).context("recreating swapchaing image view")
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        let resolution = swapchain_images[0].dimensions().width_height();
        self.viewport.dimensions = [resolution[0] as f32, resolution[1] as f32];

        self.g_buffer_normal = create_g_buffer(
            &self.memory_allocator,
            swapchain_images[0].dimensions().width_height(),
            G_BUFFER_FORMAT_NORMAL,
        )?;
        self.g_buffer_primitive_id = create_g_buffer(
            &self.memory_allocator,
            swapchain_images[0].dimensions().width_height(),
            G_BUFFER_FORMAT_PRIMITIVE_ID,
        )?;

        self.framebuffers = create_framebuffers(
            self.render_pass.clone(),
            &self.swapchain_image_views,
            self.g_buffer_normal.clone(),
            self.g_buffer_primitive_id.clone(),
        )?;

        self.lighting_pass.update_g_buffers(
            &self.descriptor_allocator,
            self.g_buffer_normal.clone(),
            self.g_buffer_primitive_id.clone(),
        )?;

        self.is_srgb_framebuffer = is_srgb_framebuffer(swapchain_images[0].clone());

        self.recreate_swapchain = false;
        Ok(())
    }

    /// Checks for submission finish and free locks on gpu resources
    fn wait_and_unlock_previous_frame(&mut self) {
        if let Some(future_previous_frame) = self.future_previous_frame.as_mut() {
            future_previous_frame.cleanup_finished();
        }
    }
}
