use super::{
    config_renderer::{ENABLE_VULKAN_VALIDATION, TIMEOUT_NANOSECS},
    debug_callback::log_vulkan_debug_callback,
    geometry_pass::GeometryPass,
    gui_pass::GuiPass,
    lighting_pass::LightingPass,
    shader_interfaces::uniform_buffers::CameraUniformBuffer,
    vulkan_init::{
        choose_physical_device_and_queue_families, create_camera_ubo, create_clear_values,
        create_depth_buffer, create_framebuffers, create_normal_buffer, create_render_pass,
        create_swapchain, create_swapchain_image_views, swapchain_properties,
        ChoosePhysicalDeviceReturn, CreateDeviceAndQueuesReturn,
    },
};
use crate::{
    engine::object::{object::ObjectId, objects_delta::ObjectsDelta, primitive_op::PrimitiveOpId},
    helper::anyhow_panic::{log_anyhow_error_and_sources, log_error_sources},
    renderer::{
        config_renderer::MINIMUM_FRAMEBUFFER_COUNT,
        vulkan_init::{
            choose_depth_buffer_format, create_command_pool, create_cpu_read_staging_buffer,
            create_device_and_queue, create_entry, create_instance, create_primitive_id_buffers,
            create_render_command_buffers, shaders_should_write_linear_color,
        },
    },
    user_interface::camera::Camera,
};
use anyhow::Context;
use ash::{extensions::khr::Synchronization2, vk};
use bort_vk::{
    Buffer, CommandBuffer, CommandPool, DebugCallback, DebugCallbackProperties, Device, Fence,
    Framebuffer, Image, ImageAccess, ImageView, Instance, MemoryAllocator, Queue, RenderPass,
    Semaphore, Surface, Swapchain, SwapchainImage,
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

    render_queue: Arc<Queue>,
    transfer_queue: Arc<Queue>,

    memory_allocator: Arc<MemoryAllocator>,
    command_pool_render: Arc<CommandPool>,
    command_pool_transfer: Arc<CommandPool>,

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

    buffer_read_resources: BufferReadResources,
}

// Public functions

impl RenderManager {
    /// Initializes Vulkan resources. If renderer fails to initiver_minoralize, returns a string explanation.
    pub fn new(window: Arc<Window>, scale_factor: f32) -> anyhow::Result<Self> {
        let entry = create_entry()?;

        // create vulkan instance
        let instance = create_instance(entry.clone(), &window)?;

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
            transfer_queue,
        } = create_device_and_queue(
            physical_device.clone(),
            debug_callback.clone(),
            render_queue_family_index,
            transfer_queue_family_index,
        )?;

        let memory_allocator = Arc::new(MemoryAllocator::new(device.clone())?);

        let command_pool_render = create_command_pool(device.clone(), &render_queue)?;
        let command_pool_transfer = create_command_pool(device.clone(), &transfer_queue)?;

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

        let mut swapchain_image_views = create_swapchain_image_views(&swapchain)?;

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
            &mut swapchain_image_views,
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
            transfer_queue_family_index,
            render_queue_family_index,
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
            command_pool_render.clone(),
            command_pool_transfer.clone(),
            scale_factor,
        )?;

        let render_command_buffers = create_render_command_buffers(
            command_pool_render.clone(),
            swapchain_image_views.len() as u32,
        )?;

        let previous_render_fence =
            Arc::new(Fence::new_signalled(device.clone()).context("creating fence")?);
        let next_frame_wait_semaphore =
            Arc::new(Semaphore::new(device.clone()).context("creating per-frame semaphore")?);
        let swapchain_image_available_semaphore = Arc::new(
            Semaphore::new(device.clone()).context("creating per-swapchain-image semaphore")?,
        );

        let buffer_read_resources = BufferReadResources::new(
            device.clone(),
            &command_pool_transfer,
            &command_pool_render,
            memory_allocator.clone(),
        )?;

        Ok(Self {
            instance,
            debug_callback,
            device,

            render_queue,
            transfer_queue,

            memory_allocator,
            command_pool_render,
            command_pool_transfer,

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

            buffer_read_resources,
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

    #[inline]
    pub fn update_objects(&mut self, objects_delta: ObjectsDelta) -> anyhow::Result<()> {
        self.geometry_pass
            .update_objects(objects_delta, &self.transfer_queue, &self.render_queue)
    }

    pub fn update_gui_textures(
        &mut self,
        textures_delta: Vec<TexturesDelta>,
    ) -> anyhow::Result<()> {
        self.wait_for_previous_frame_fence()?;

        self.gui_pass
            .update_textures(textures_delta, &self.transfer_queue, &self.render_queue)?;

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

        let framebuffer_index = self
            .current_framebuffer_index(self.framebuffer_index_last_rendered_to, swapchain_index);

        // record commands

        let command_buffer = self.render_command_buffers[framebuffer_index].clone();
        self.record_render_commands(&command_buffer, framebuffer_index)?;

        // submit commands

        self.previous_render_fence
            .reset()
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
            self.device.inner().queue_submit(
                self.render_queue.handle(),
                &[submit_info.build()],
                self.previous_render_fence.handle(),
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

    pub fn get_element_at_screen_coordinate(
        &mut self,
        screen_coordinate: [f32; 2],
    ) -> anyhow::Result<ElementAtPoint> {
        self.copy_primitive_id_at_screen_coordinate_to_buffer(screen_coordinate)?;
        self.read_object_id_from_buffer()
    }

    pub fn wait_idle_device(&self) -> anyhow::Result<()> {
        self.device.wait_idle().context("calling vkDeviceWaitIdle")
    }
}

// Private functions

/// If there is only one swapchain image we create two framebuffers so we can access the previous
/// render for some images.
fn determine_framebuffer_count(
    swapchain_image_views: &Vec<Arc<ImageView<SwapchainImage>>>,
) -> usize {
    swapchain_image_views.len().max(MINIMUM_FRAMEBUFFER_COUNT)
}

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
            &mut self.swapchain_image_views,
            &self.normal_buffer,
            &self.primitive_id_buffers,
            &self.depth_buffer,
        )?;

        self.lighting_pass
            .update_g_buffers(&self.normal_buffer, &self.primitive_id_buffers)?;

        Ok(())
    }

    fn reset_render_command_buffers(&self) -> anyhow::Result<()> {
        self.render_queue
            .wait_idle()
            .context("calling vkQueueWaitIdle for render queue")?;

        for command_buffer in &self.render_command_buffers {
            unsafe {
                self.device.inner().reset_command_buffer(
                    command_buffer.handle(),
                    vk::CommandBufferResetFlags::empty(),
                )
            }
            .context("resetting render command buffers")?;
        }
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
        framebuffer_index: usize,
    ) -> anyhow::Result<()> {
        let command_buffer_handle = command_buffer.handle();
        let device_ash = self.device.inner();

        let viewport = self.framebuffers[framebuffer_index].whole_viewport();
        let rect_2d = self.framebuffers[framebuffer_index].whole_rect();

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe { device_ash.begin_command_buffer(command_buffer_handle, &begin_info) }
            .context("beinning render command buffer recording")?;

        let render_pass_begin = vk::RenderPassBeginInfo::builder()
            .render_pass(self.render_pass.handle())
            .framebuffer(self.framebuffers[framebuffer_index].handle())
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

        self.lighting_pass.record_commands(
            framebuffer_index,
            &command_buffer,
            viewport,
            rect_2d,
        )?;

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

    /// Determines the new framebuffer index
    fn current_framebuffer_index(
        &self,
        previous_framebuffer_index: usize,
        swapchain_index: usize,
    ) -> usize {
        if self.swapchain_image_views.len() == 1 {
            return (previous_framebuffer_index + 1) % MINIMUM_FRAMEBUFFER_COUNT;
        }
        return swapchain_index;
    }

    fn copy_primitive_id_at_screen_coordinate_to_buffer(
        &self,
        screen_coordinate: [f32; 2],
    ) -> Result<(), anyhow::Error> {
        let different_queue_family_indices =
            self.render_queue.family_index() != self.transfer_queue.family_index();

        if different_queue_family_indices {
            // render queue release operation
            self.record_and_submit_pre_transfer_sync_commands()?;
        }

        let command_buffer_transfer = &self.buffer_read_resources.command_buffer_transfer;

        self.record_primitive_id_copy_commands(
            screen_coordinate,
            command_buffer_transfer,
            different_queue_family_indices,
        )?;
        self.submit_primitive_id_copy_commands(
            command_buffer_transfer,
            different_queue_family_indices,
        )?;

        if different_queue_family_indices {
            // render queue release operation
            self.record_and_submit_post_transfer_sync_commands()?;
        }

        Ok(())
    }

    fn submit_primitive_id_copy_commands(
        &self,
        command_buffer_transfer: &Arc<CommandBuffer>,
        different_queue_family_indices: bool,
    ) -> anyhow::Result<()> {
        let semaphores_before_transfer = [self
            .buffer_read_resources
            .semaphore_before_transfer
            .handle()];
        let semaphores_after_transfer =
            [self.buffer_read_resources.semaphore_after_transfer.handle()];

        let transfer_submit_command_buffers = [command_buffer_transfer.handle()];

        let mut transfer_submit_info =
            vk::SubmitInfo::builder().command_buffers(&transfer_submit_command_buffers);
        if different_queue_family_indices {
            transfer_submit_info = transfer_submit_info
                .wait_semaphores(&semaphores_before_transfer)
                .wait_dst_stage_mask(&[vk::PipelineStageFlags::TRANSFER])
                .signal_semaphores(&semaphores_after_transfer);
        }

        self.buffer_read_resources
            .completion_fence
            .reset()
            .context("resetting primitive id buffer reset fn")?;

        unsafe {
            self.device
                .inner()
                .queue_submit(
                    self.transfer_queue.handle(),
                    &[transfer_submit_info.build()],
                    self.buffer_read_resources.completion_fence.handle(),
                )
                .context("submitting commands to read primitive id at coordinate")?;
        }

        Ok(())
    }

    // todo move to BufferReadResources and just have submit as sub-function
    fn record_primitive_id_copy_commands(
        &self,
        screen_coordinate: [f32; 2],
        command_buffer_transfer: &CommandBuffer,
        different_queue_family_indices: bool,
    ) -> anyhow::Result<()> {
        let last_primitive_id_buffer =
            self.primitive_id_buffers[self.framebuffer_index_last_rendered_to].clone();

        let image_subresource_range = vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            level_count: 1,
            layer_count: 1,
            ..Default::default()
        };

        let image_subresource_layers = vk::ImageSubresourceLayers {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            layer_count: 1,
            ..Default::default()
        };

        let image_offset = vk::Offset3D {
            x: screen_coordinate[0].round() as i32,
            y: screen_coordinate[1].round() as i32,
            z: 0,
        };

        let image_extent = vk::Extent3D {
            width: 1,
            height: 1,
            depth: 1,
        };

        let buffer_image_copy_region = vk::BufferImageCopy {
            buffer_offset: 0,
            image_subresource: image_subresource_layers,
            image_offset,
            image_extent,
            ..Default::default()
        };

        let image_barrier_before_transfer = vk::ImageMemoryBarrier::builder()
            .src_access_mask(vk::AccessFlags::SHADER_READ)
            .dst_access_mask(vk::AccessFlags::TRANSFER_READ)
            .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
            .image(last_primitive_id_buffer.image().handle())
            .subresource_range(image_subresource_range)
            .src_queue_family_index(self.render_queue.family_index())
            .dst_queue_family_index(self.transfer_queue.family_index());

        let image_barrier_after_transfer = vk::ImageMemoryBarrier::builder()
            .src_access_mask(vk::AccessFlags::TRANSFER_READ)
            .dst_access_mask(vk::AccessFlags::SHADER_WRITE)
            .old_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
            .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .image(last_primitive_id_buffer.image().handle())
            .subresource_range(image_subresource_range)
            .src_queue_family_index(self.transfer_queue.family_index())
            .dst_queue_family_index(self.render_queue.family_index());

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        command_buffer_transfer
            .begin(&begin_info)
            .context("beginning command buffer get_element_at_screen_coordinate")?;

        let device_ash = self.device.inner();
        unsafe {
            let command_buffer_handle = command_buffer_transfer.handle();

            let src_stage_mask = if different_queue_family_indices {
                vk::PipelineStageFlags::BOTTOM_OF_PIPE // this is a queue aquire operation https://registry.khronos.org/vulkan/specs/1.3-extensions/html/vkspec.html#VkImageMemoryBarrier
            } else {
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
            };
            device_ash.cmd_pipeline_barrier(
                command_buffer_handle,
                src_stage_mask,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[], // don't worry about buffer memory barriers because this funciton should be the only code that touches it and there's a fence wait after this
                &[image_barrier_before_transfer.build()],
            );

            device_ash.cmd_copy_image_to_buffer(
                command_buffer_handle,
                last_primitive_id_buffer.image().handle(),
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                self.buffer_read_resources.cpu_read_staging_buffer.handle(),
                &[buffer_image_copy_region],
            );

            let dst_stage_mask = if different_queue_family_indices {
                vk::PipelineStageFlags::TOP_OF_PIPE // this is a queue release operation https://registry.khronos.org/vulkan/specs/1.3-extensions/html/vkspec.html#VkImageMemoryBarrier
            } else {
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
            };
            device_ash.cmd_pipeline_barrier(
                command_buffer_handle,
                vk::PipelineStageFlags::TRANSFER,
                dst_stage_mask,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[image_barrier_after_transfer.build()],
            );
        }

        command_buffer_transfer
            .end()
            .context("ending command buffer get_element_at_screen_coordinate")?;

        Ok(())
    }

    fn record_and_submit_pre_transfer_sync_commands(&self) -> anyhow::Result<()> {
        let device_ash = self.device.inner();

        let semaphores_before_transfer = [self
            .buffer_read_resources
            .semaphore_before_transfer
            .handle()];

        let last_primitive_id_buffer =
            self.primitive_id_buffers[self.framebuffer_index_last_rendered_to].clone();

        let image_subresource_range = vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            level_count: 1,
            layer_count: 1,
            ..Default::default()
        };

        // render queue release
        let image_barrier_after_render = vk::ImageMemoryBarrier::builder()
            .src_access_mask(vk::AccessFlags::SHADER_READ)
            .dst_access_mask(vk::AccessFlags::TRANSFER_READ)
            .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
            .image(last_primitive_id_buffer.image().handle())
            .subresource_range(image_subresource_range)
            .src_queue_family_index(self.render_queue.family_index())
            .dst_queue_family_index(self.transfer_queue.family_index());

        let command_buffer_render_sync = self
            .buffer_read_resources
            .command_buffer_post_render_sync
            .clone();

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        command_buffer_render_sync
            .begin(&begin_info)
            .context("beginning command buffer record_and_submit_pre_transfer_sync_commands")?;

        unsafe {
            device_ash.cmd_pipeline_barrier(
                command_buffer_render_sync.handle(),
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[image_barrier_after_render.build()],
            );
        }

        command_buffer_render_sync
            .end()
            .context("ending command buffer record_and_submit_pre_transfer_sync_commands")?;

        let submit_command_buffers = [command_buffer_render_sync.handle()];
        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(&submit_command_buffers)
            .signal_semaphores(&semaphores_before_transfer);

        unsafe {
            device_ash
                .queue_submit(
                    self.render_queue.handle(),
                    &[submit_info.build()],
                    vk::Fence::null(),
                )
                .context("submitting commands to sync reading primitive id at a coordinate")?;
        }

        Ok(())
    }

    fn record_and_submit_post_transfer_sync_commands(&self) -> anyhow::Result<()> {
        let device_ash = self.device.inner();

        let semaphores_after_transfer = [self
            .buffer_read_resources
            .semaphore_before_transfer
            .handle()];

        let last_primitive_id_buffer =
            self.primitive_id_buffers[self.framebuffer_index_last_rendered_to].clone();

        let image_subresource_range = vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            level_count: 1,
            layer_count: 1,
            ..Default::default()
        };

        // render queue aquire
        let image_barrier_before_render = vk::ImageMemoryBarrier::builder()
            .src_access_mask(vk::AccessFlags::TRANSFER_READ)
            .dst_access_mask(vk::AccessFlags::SHADER_WRITE)
            .old_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
            .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .image(last_primitive_id_buffer.image().handle())
            .subresource_range(image_subresource_range)
            .src_queue_family_index(self.transfer_queue.family_index())
            .dst_queue_family_index(self.render_queue.family_index());

        let command_buffer_render_sync = self
            .buffer_read_resources
            .command_buffer_pre_render_sync
            .clone();

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        command_buffer_render_sync
            .begin(&begin_info)
            .context("beginning command buffer record_and_submit_pre_transfer_sync_commands")?;

        unsafe {
            device_ash.cmd_pipeline_barrier(
                command_buffer_render_sync.handle(),
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[image_barrier_before_render.build()],
            );
        }

        command_buffer_render_sync
            .end()
            .context("ending command buffer record_and_submit_pre_transfer_sync_commands")?;

        let submit_command_buffers = [command_buffer_render_sync.handle()];
        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(&submit_command_buffers)
            .wait_semaphores(&semaphores_after_transfer)
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT]);

        unsafe {
            device_ash
                .queue_submit(
                    self.render_queue.handle(),
                    &[submit_info.build()],
                    vk::Fence::null(),
                )
                .context("submitting commands to sync reading primitive id at a coordinate")?;
        }

        Ok(())
    }

    fn read_object_id_from_buffer(&mut self) -> anyhow::Result<ElementAtPoint> {
        self.buffer_read_resources
            .completion_fence
            .wait(TIMEOUT_NANOSECS)
            .context("waiting for render id buffer copy fence")?;

        let rendered_id = self
            .buffer_read_resources
            .cpu_read_staging_buffer
            .memory_allocation_mut()
            .read_struct::<u32>(0)
            .context("reading render id")?;

        Ok(ElementAtPoint::from_rendered_id(rendered_id))
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
                self.command_pool_transfer.handle(),
                vk::CommandPoolResetFlags::RELEASE_RESOURCES,
            )
        };
        if let Err(e) = command_pool_reset_res {
            log_error_sources(&e, 0);
        }

        let command_pool_reset_res = unsafe {
            self.device.inner().reset_command_pool(
                self.command_pool_render.handle(),
                vk::CommandPoolResetFlags::RELEASE_RESOURCES,
            )
        };
        if let Err(e) = command_pool_reset_res {
            log_error_sources(&e, 0);
        }
    }
}

// ~~ Helper Structs ~~

pub enum ElementAtPoint {
    Object {
        object_id: ObjectId,
        primitive_op_id: PrimitiveOpId,
    },
    Background,
    // X, Y, Z manilulation ui elements
}

impl ElementAtPoint {
    pub fn from_rendered_id(rendered_id: u32) -> Self {
        match rendered_id {
            0 => Self::Background,
            encoded_id => {
                let object_id_u32 = encoded_id << 16;
                let object_id = ObjectId::from(object_id_u32 as usize);

                let primitive_op_id_u32 = encoded_id & 0x0000FFFF;
                let primitive_op_id = PrimitiveOpId::from(encoded_id as usize);

                Self::Object {
                    object_id,
                    primitive_op_id,
                }
            }
        }
    }
}

struct BufferReadResources {
    pub command_buffer_transfer: Arc<CommandBuffer>,
    pub command_buffer_post_render_sync: Arc<CommandBuffer>,
    pub command_buffer_pre_render_sync: Arc<CommandBuffer>,
    pub completion_fence: Arc<Fence>,
    pub semaphore_before_transfer: Arc<Semaphore>,
    pub semaphore_after_transfer: Arc<Semaphore>,
    pub cpu_read_staging_buffer: Buffer,
}

impl BufferReadResources {
    pub fn new(
        device: Arc<Device>,
        command_pool_transfer_queue: &Arc<CommandPool>,
        command_pool_render_queue: &Arc<CommandPool>,
        memory_allocator: Arc<MemoryAllocator>,
    ) -> anyhow::Result<Self> {
        let command_buffer_transfer = Arc::new(
            command_pool_transfer_queue
                .allocate_command_buffer(vk::CommandBufferLevel::PRIMARY)
                .context("allocating buffer read transfer queue command buffer")?,
        );

        let command_buffer_post_render_sync = Arc::new(
            command_pool_render_queue
                .allocate_command_buffer(vk::CommandBufferLevel::PRIMARY)
                .context("allocating buffer read render sync command buffer")?,
        );

        let command_buffer_pre_render_sync = Arc::new(
            command_pool_render_queue
                .allocate_command_buffer(vk::CommandBufferLevel::PRIMARY)
                .context("allocating buffer read render sync command buffer")?,
        );

        let completion_fence =
            Arc::new(Fence::new_unsignalled(device.clone()).context("creating fence")?);

        let semaphore_before_transfer =
            Arc::new(Semaphore::new(device.clone()).context("creating semaphore")?);
        let semaphore_after_transfer =
            Arc::new(Semaphore::new(device.clone()).context("creating semaphore")?);

        let cpu_read_staging_buffer = create_cpu_read_staging_buffer(memory_allocator)?;

        Ok(Self {
            command_buffer_transfer,
            command_buffer_post_render_sync,
            command_buffer_pre_render_sync,
            completion_fence,
            semaphore_before_transfer,
            semaphore_after_transfer,
            cpu_read_staging_buffer,
        })
    }
}
