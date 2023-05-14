use super::{
    config_renderer::{ENABLE_VULKAN_VALIDATION, TIMEOUT_NANOSECS, VULKAN_VER_MAJ, VULKAN_VER_MIN},
    debug_callback::log_vulkan_debug_callback,
    geometry_pass::GeometryPass,
    gui_pass::GuiPass,
    lighting_pass::LightingPass,
    shader_interfaces::uniform_buffers::CameraUniformBuffer,
    vulkan_init::{
        choose_physical_device_and_queue_families, create_camera_ubo, create_clear_values,
        create_depth_buffer, create_framebuffers, create_normal_buffer, create_per_frame_fence,
        create_render_pass, create_swapchain, create_swapchain_image_views, swapchain_properties,
        ChoosePhysicalDeviceReturn, CreateDeviceAndQueuesReturn,
    },
};
use crate::{
    config::ENGINE_NAME,
    engine::object::{object_collection::ObjectCollection, objects_delta::ObjectsDelta},
    helper::anyhow_panic::{log_anyhow_error_and_sources, log_error_sources},
    renderer::{
        config_renderer::MINIMUM_FRAMEBUFFER_COUNT,
        vulkan_init::{
            choose_depth_buffer_format, create_command_pool, create_device_and_queue, create_entry,
            create_primitive_id_buffers, create_render_command_buffers,
            shaders_should_write_linear_color,
        },
    },
    user_interface::camera::Camera,
};
use anyhow::Context;
use ash::vk;
use bort::{
    ApiVersion, Buffer, CommandBuffer, CommandPool, DebugCallback, DebugCallbackProperties, Device,
    Fence, Framebuffer, Image, ImageView, Instance, MemoryAllocator, Queue, RenderPass, Semaphore,
    Surface, Swapchain, SwapchainImage,
};
use egui::{ClippedPrimitive, TexturesDelta};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::sync::Arc;
use winit::window::Window;

/// Contains Vulkan resources and methods to manage rendering
pub struct RenderManager {
    instance: Arc<Instance>,
    debug_callback: Option<Arc<DebugCallback>>,

    device: Arc<Device>,
    render_queue: Queue,

    memory_allocator: Arc<MemoryAllocator>,
    command_pool: Arc<CommandPool>,

    window: Arc<Window>,
    surface: Arc<Surface>,
    swapchain: Arc<Swapchain>,
    swapchain_image_views: Vec<Arc<ImageView<SwapchainImage>>>,
    shaders_write_linear_color: bool,

    render_pass: Arc<RenderPass>,
    /// One per swapchain image, or two if there's only one swapchain image so we can store some
    /// images written to in the previous render
    framebuffers: Vec<Arc<Framebuffer>>,
    /// One for each framebuffer attachment
    clear_values: Vec<vk::ClearValue>,

    depth_buffer: Arc<ImageView<Image>>,
    normal_buffer: Arc<ImageView<Image>>,
    /// One per framebuffer
    primitive_id_buffers: Vec<Arc<ImageView<Image>>>,
    camera_ubo: Arc<Buffer>,

    lighting_pass: LightingPass,
    geometry_pass: GeometryPass,
    gui_pass: GuiPass,

    /// One per framebuffer
    render_command_buffers: Vec<Arc<CommandBuffer>>,
    previous_render_fence: Arc<Fence>,
    next_frame_wait_semaphore: Arc<Semaphore>,
    swapchain_image_available_semaphore: Arc<Semaphore>,

    /// Indicates which framebuffer is being processed right now.
    framebuffer_index_currently_rendering: usize,
    /// Indicates which framebuffer was rendered to in the previous frame.
    framebuffer_index_last_rendered_to: usize,
    /// Can be set to true with [`Self::set_window_just_resized_flag`] and set to false in [`Self::render_frame`]
    window_just_resized: bool,
}

// Public functions

impl RenderManager {
    /// Initializes Vulkan resources. If renderer fails to initiver_minoralize, returns a string explanation.
    pub fn new(window: Arc<Window>, scale_factor: f32) -> anyhow::Result<Self> {
        let entry = create_entry()?;

        // create vulkan instance
        let api_version = ApiVersion::new(VULKAN_VER_MAJ, VULKAN_VER_MIN);
        let instance = Arc::new(
            Instance::new(
                entry.clone(),
                api_version,
                ENGINE_NAME,
                window.raw_display_handle(),
                ENABLE_VULKAN_VALIDATION,
                [],
                [],
            )
            .context("creating vulkan instance")?,
        );
        info!(
            "created vulkan instance. api version = {:?}",
            instance.api_version()
        );

        // setup validation layer debug callback
        let debug_callback_properties = DebugCallbackProperties::default();
        let debug_callback = if ENABLE_VULKAN_VALIDATION {
            match DebugCallback::new(
                instance.clone(),
                Some(log_vulkan_debug_callback),
                debug_callback_properties,
            ) {
                Ok(x) => {
                    info!("enabling vulkan validation layers and debug callback");
                    Some(Arc::new(x))
                }
                Err(e) => {
                    warn!("validation layer debug callback requested but cannot be setup due to: {:?}", e);
                    None
                }
            }
        } else {
            debug!("vulkan validation layers disabled");
            None
        };

        let surface = Arc::new(
            Surface::new(
                &entry,
                instance.clone(),
                window.raw_display_handle(),
                window.raw_window_handle(),
            )
            .context("creating vulkan surface")?,
        );

        let ChoosePhysicalDeviceReturn {
            physical_device,
            render_queue_family_index,
            transfer_queue_family_index,
        } = choose_physical_device_and_queue_families(instance.clone(), &surface)?;
        let physical_device = Arc::new(physical_device);
        info!(
            "using vulkan physical device: {} (type: {:?})",
            physical_device.name(),
            physical_device.properties().device_type,
        );
        debug!("render queue family index = {}", render_queue_family_index);
        debug!(
            "transfer queue family index = {}",
            transfer_queue_family_index
        );

        let CreateDeviceAndQueuesReturn {
            device,
            render_queue,
        } = create_device_and_queue(
            physical_device.clone(),
            debug_callback.clone(),
            render_queue_family_index,
        )?;

        let memory_allocator = Arc::new(MemoryAllocator::new(device.clone())?);

        let command_pool = create_command_pool(device.clone(), &render_queue)?;

        let swapchain = create_swapchain(device.clone(), surface.clone(), &window)?;
        debug!(
            "swapchain surface format = {:?}",
            swapchain.properties().surface_format
        );
        debug!(
            "swapchain present mode = {:?}",
            swapchain.properties().present_mode
        );
        debug!(
            "swapchain composite alpha = {:?}",
            swapchain.properties().composite_alpha
        );
        let shaders_write_linear_color =
            shaders_should_write_linear_color(swapchain.properties().surface_format);

        let swapchain_image_views = create_swapchain_image_views(&swapchain)?;

        let framebuffer_count = determine_framebuffer_count(&swapchain_image_views);

        let depth_buffer_format = choose_depth_buffer_format(&physical_device)?;

        let render_pass =
            create_render_pass(device.clone(), swapchain.properties(), depth_buffer_format)?;

        let depth_buffer = create_depth_buffer(
            memory_allocator.clone(),
            swapchain.properties().dimensions(),
            depth_buffer_format,
        )?;

        let normal_buffer = create_normal_buffer(
            memory_allocator.clone(),
            swapchain.properties().dimensions(),
        )?;

        let primitive_id_buffers = create_primitive_id_buffers(
            framebuffer_count,
            memory_allocator.clone(),
            swapchain.properties().dimensions(),
        )?;

        let camera_ubo = create_camera_ubo(memory_allocator.clone())?;

        let framebuffers = create_framebuffers(
            framebuffer_count,
            &render_pass,
            &swapchain_image_views,
            &normal_buffer,
            &primitive_id_buffers,
            &depth_buffer,
        )?;

        let clear_values = create_clear_values();

        let geometry_pass = GeometryPass::new(
            device.clone(),
            memory_allocator.clone(),
            &render_pass,
            &camera_ubo,
            render_queue.famliy_index(),
        )?;

        let lighting_pass = LightingPass::new(
            device.clone(),
            &render_pass,
            &camera_ubo,
            normal_buffer.as_ref(),
            &primitive_id_buffers,
        )?;

        let gui_pass = GuiPass::new(
            device.clone(),
            memory_allocator.clone(),
            &render_pass,
            render_queue_family_index,
            scale_factor,
        )?;

        let render_command_buffers = create_render_command_buffers(
            command_pool.clone(),
            swapchain_image_views.len() as u32,
        )?;

        let previous_render_fence = create_per_frame_fence(device.clone())?;
        let next_frame_wait_semaphore =
            Arc::new(Semaphore::new(device.clone()).context("creating per-frame semaphore")?);
        let swapchain_image_available_semaphore = Arc::new(
            Semaphore::new(device.clone()).context("creating per-swapchain-image semaphore")?,
        );

        Ok(Self {
            instance,
            debug_callback,

            device,
            render_queue,

            memory_allocator,
            command_pool,

            window,
            surface,
            swapchain,
            swapchain_image_views,
            shaders_write_linear_color,

            render_pass,
            framebuffers,
            clear_values,

            depth_buffer,
            normal_buffer,
            primitive_id_buffers,
            camera_ubo,

            geometry_pass,
            lighting_pass,
            gui_pass,

            render_command_buffers,
            previous_render_fence,
            next_frame_wait_semaphore,
            swapchain_image_available_semaphore,

            framebuffer_index_currently_rendering: 0,
            framebuffer_index_last_rendered_to: 0,
            window_just_resized: false,
        })
    }

    pub fn update_camera(&mut self, camera: &Camera) -> anyhow::Result<()> {
        self.wait_idle_device()?;

        let dimensions = self.swapchain.properties().width_height;
        let camera_data = CameraUniformBuffer::from_camera(
            camera,
            [dimensions[0] as f32, dimensions[1] as f32],
            self.shaders_write_linear_color,
        );

        let camera_ubo_mut = match Arc::get_mut(&mut self.camera_ubo) {
            Some(ubo) => ubo,
            None => {
                warn!("attempted to borrow camera buffer as mutable but couldn't! skipping update_camera()...");
                return Ok(());
            }
        };

        camera_ubo_mut
            .write_struct(camera_data, 0)
            .context("uploading camera ubo data")?;

        Ok(())
    }

    pub fn update_object_buffers(
        &mut self,
        object_collection: &ObjectCollection,
        objects_delta: ObjectsDelta,
    ) -> anyhow::Result<()> {
        self.geometry_pass.update_object_buffers(
            object_collection,
            objects_delta,
            &self.render_queue,
        )
    }

    pub fn update_gui_textures(
        &mut self,
        textures_delta: Vec<TexturesDelta>,
    ) -> anyhow::Result<()> {
        self.wait_for_previous_frame_fence()?;

        self.gui_pass
            .update_textures(textures_delta, &self.render_queue)?;

        Ok(())
    }

    pub fn set_scale_factor(&mut self, scale_factor: f32) {
        self.gui_pass.set_scale_factor(scale_factor);
    }

    pub fn set_gui_primitives(&mut self, gui_primitives: Vec<ClippedPrimitive>) {
        self.gui_pass.set_gui_primitives(gui_primitives);
    }

    pub fn set_window_just_resized_flag(&mut self) {
        self.window_just_resized = true;
    }

    /// Submits Vulkan commands for rendering a frame.
    pub fn render_frame(&mut self) -> anyhow::Result<()> {
        // wait for previous frame render/resource upload to finish

        self.wait_for_previous_frame_fence()?;
        // previous frame confirmed finished rendering
        self.framebuffer_index_last_rendered_to = self.framebuffer_index_currently_rendering;

        self.gui_pass.free_previous_vertex_and_index_buffers();

        // note: I found that this check is needed on wayland because the later commands weren't returning 'out of date'...
        if self.window_just_resized {
            self.window_just_resized = false;
            self.recreate_swapchain()?;
        }

        // aquire next swapchain image

        let aquire_res = self.swapchain.aquire_next_image(
            TIMEOUT_NANOSECS,
            Some(&self.swapchain_image_available_semaphore),
            None,
        );

        if let Err(aquire_err) = aquire_res {
            if aquire_err == vk::Result::ERROR_OUT_OF_DATE_KHR {
                debug!("out of date swapchain on aquire");
                return self.recreate_swapchain();
            } else {
                return Err(aquire_err).context("calling vkAcquireNextImageKHR");
            }
        }

        let (swapchain_index, swapchain_is_suboptimal) =
            aquire_res.expect("handled err case in previous lines");
        let swapchain_index = swapchain_index as usize;
        if swapchain_is_suboptimal {
            debug!("suboptimal swapchain");
            return self.recreate_swapchain();
        }

        // record commands

        let command_buffer = self.render_command_buffers[swapchain_index].clone();
        self.record_render_commands(&command_buffer, swapchain_index)?;

        // submit commands

        let device_ash = self.device.inner();
        let previous_render_fence_handle = self.previous_render_fence.handle();
        unsafe { device_ash.reset_fences(&[previous_render_fence_handle]) }
            .context("reseting previous render fence")?;

        let submit_command_buffers = [command_buffer.handle()];

        let wait_semaphores = [self.swapchain_image_available_semaphore.handle()];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];

        let signal_semaphores = [self.next_frame_wait_semaphore.handle()];

        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(&submit_command_buffers)
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .signal_semaphores(&signal_semaphores);

        unsafe {
            device_ash.queue_submit(
                self.render_queue.handle(),
                &[submit_info.build()],
                previous_render_fence_handle,
            )
        }
        .context("submitting render commands")?;

        self.framebuffer_index_currently_rendering = swapchain_index;

        // submit present instruction

        let swapchain_present_indices = [swapchain_index as u32];
        let swapchain_handles = [self.swapchain.handle()];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(&signal_semaphores)
            .image_indices(&swapchain_present_indices)
            .swapchains(&swapchain_handles);

        let present_res = unsafe {
            self.swapchain
                .swapchain_loader()
                .queue_present(self.render_queue.handle(), &present_info)
        };

        if let Err(present_err) = present_res {
            if present_err == vk::Result::ERROR_OUT_OF_DATE_KHR
                || present_err == vk::Result::SUBOPTIMAL_KHR
            {
                debug!("out of date or suboptimal swapchain upon present");
                self.recreate_swapchain()?;
            } else {
                return Err(present_err).context("submitting swapchain present instruction")?;
            }
        }

        Ok(())
    }

    pub fn wait_idle_device(&self) -> anyhow::Result<()> {
        self.device.wait_idle().context("calling vkDeviceWaitIdle")
    }

    pub fn reset_render_command_buffers(&self) -> anyhow::Result<()> {
        self.wait_idle_device()?;

        for command_buffer in &self.render_command_buffers {
            unsafe {
                self.device.inner().reset_command_buffer(
                    command_buffer.handle(),
                    vk::CommandBufferResetFlags::empty(),
                )
            }
            .context("resetting render command pool")?;
        }
        Ok(())
    }
}

/// If there is only one swapchain image we create two framebuffers so we can access the previous
/// render for some images.
fn determine_framebuffer_count(
    swapchain_image_views: &Vec<Arc<ImageView<SwapchainImage>>>,
) -> usize {
    swapchain_image_views.len().max(MINIMUM_FRAMEBUFFER_COUNT)
}

// Private functions

impl RenderManager {
    /// Recreates the swapchain, g-buffers and assiciated descriptor sets, then unsets `recreate_swapchain` trigger.
    fn recreate_swapchain(&mut self) -> anyhow::Result<()> {
        trace!("recreating swapchain...");

        // do host-device sync and reset command buffers
        self.reset_render_command_buffers()?;

        // clean up resources depending on the swapchain
        self.framebuffers.clear();
        self.swapchain_image_views.clear();

        // recreate the swapchain
        let swapchain_properties = swapchain_properties(&self.device, &self.surface, &self.window)?;
        trace!(
            "creating swapchain with dimensions: {:?}",
            swapchain_properties.width_height
        );
        self.swapchain = self
            .swapchain
            .recreate_replace(swapchain_properties)
            .context("recreating swapchain")?;

        // reinitialize related resources
        self.shaders_write_linear_color =
            shaders_should_write_linear_color(self.swapchain.properties().surface_format);
        self.swapchain_image_views = create_swapchain_image_views(&self.swapchain)?;

        let framebuffer_count = determine_framebuffer_count(&self.swapchain_image_views);

        let depth_buffer_format = self.depth_buffer.image().properties().format;

        self.render_pass = create_render_pass(
            self.device.clone(),
            self.swapchain.properties(),
            depth_buffer_format,
        )?;

        self.normal_buffer = create_normal_buffer(
            self.memory_allocator.clone(),
            self.swapchain.properties().dimensions(),
        )?;

        self.primitive_id_buffers = create_primitive_id_buffers(
            framebuffer_count,
            self.memory_allocator.clone(),
            self.swapchain.properties().dimensions(),
        )?;

        self.depth_buffer = create_depth_buffer(
            self.memory_allocator.clone(),
            self.swapchain.properties().dimensions(),
            depth_buffer_format,
        )?;

        self.framebuffers = create_framebuffers(
            framebuffer_count,
            &self.render_pass,
            &self.swapchain_image_views,
            &self.normal_buffer,
            &self.primitive_id_buffers,
            &self.depth_buffer,
        )?;

        self.lighting_pass
            .update_g_buffers(&self.normal_buffer, &self.primitive_id_buffers)?;

        Ok(())
    }

    fn wait_for_previous_frame_fence(&mut self) -> anyhow::Result<()> {
        let wait_fence_handles = [self.previous_render_fence.handle()];

        let fence_wait_res = unsafe {
            self.device
                .inner()
                .wait_for_fences(&wait_fence_handles, true, TIMEOUT_NANOSECS)
        };

        if let Err(fence_wait_err) = fence_wait_res {
            if fence_wait_err == vk::Result::TIMEOUT {
                error!(
                    "previous render fence timed out! timeout set to {}ns",
                    TIMEOUT_NANOSECS
                );
                // todo can handle this on caller side
                return Err(fence_wait_err)
                    .context("timeout while waiting for previous frame fence");
            } else {
                return Err(fence_wait_err).context("waiting for previous frame fence");
            }
        }

        Ok(())
    }

    fn record_render_commands(
        &mut self,
        command_buffer: &CommandBuffer,
        swapchain_index: usize,
    ) -> anyhow::Result<()> {
        let command_buffer_handle = command_buffer.handle();
        let device_ash = self.device.inner();

        let viewport = self.framebuffers[swapchain_index].whole_viewport();
        let rect_2d = self.framebuffers[swapchain_index].whole_rect();

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe { device_ash.begin_command_buffer(command_buffer_handle, &begin_info) }
            .context("beinning render command buffer recording")?;

        let render_pass_begin = vk::RenderPassBeginInfo::builder()
            .render_pass(self.render_pass.handle())
            .framebuffer(self.framebuffers[swapchain_index].handle())
            .render_area(rect_2d)
            .clear_values(self.clear_values.as_slice());
        unsafe {
            device_ash.cmd_begin_render_pass(
                command_buffer_handle,
                &render_pass_begin,
                vk::SubpassContents::INLINE,
            )
        };

        self.geometry_pass
            .record_commands(&command_buffer, viewport, rect_2d)?;

        unsafe { device_ash.cmd_next_subpass(command_buffer_handle, vk::SubpassContents::INLINE) };

        self.lighting_pass
            .record_commands(&command_buffer, viewport, rect_2d)?;

        self.gui_pass.record_render_commands(
            &command_buffer,
            self.shaders_write_linear_color,
            [viewport.width, viewport.height],
        )?;

        unsafe { device_ash.cmd_end_render_pass(command_buffer_handle) };

        unsafe { device_ash.end_command_buffer(command_buffer_handle) }
            .context("ending render command buffer recording")?;

        Ok(())
    }
}

impl Drop for RenderManager {
    fn drop(&mut self) {
        debug!("dropping render manager...");

        let wait_res = self.wait_idle_device();
        if let Err(e) = wait_res {
            log_anyhow_error_and_sources(&e, "renderer clean up");
        }

        let command_pool_reset_res = unsafe {
            self.device.inner().reset_command_pool(
                self.command_pool.handle(),
                vk::CommandPoolResetFlags::RELEASE_RESOURCES,
            )
        };
        if let Err(e) = command_pool_reset_res {
            log_error_sources(&e, 0);
        }
        self.render_command_buffers.clear();
    }
}
