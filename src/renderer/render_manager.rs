use super::{
    config_renderer::{RenderDebugOptions, TIMEOUT_NANOSECS},
    element_id_reader::{ElementAtPoint, ElementIdReader},
    pass_geometry::GeometryPass,
    pass_gui::GuiPass,
    pass_lighting::LightingPass,
    pass_overlay::OverlayPass,
    shader_interfaces::uniform_buffers::CameraUniformBuffer,
    vulkan_init::{
        choose_physical_device_and_queue_families, create_camera_ubo, create_clear_values,
        create_depth_buffer, create_framebuffers, create_normal_buffer, create_render_pass,
        create_swapchain, create_swapchain_image_views, swapchain_properties,
        ChoosePhysicalDeviceReturn, CreateDeviceAndQueuesReturn,
    },
};
use crate::{
    config,
    engine::object::objects_delta::ObjectsDelta,
    helper::anyhow_panic::log_anyhow_error_and_sources,
    renderer::{
        config_renderer::FRAMES_IN_FLIGHT,
        pass_gizmo::GizmoPass,
        shader_interfaces::uniform_buffers::GizmoUniformBuffer,
        vulkan_init::{
            choose_depth_buffer_format, create_albedo_buffer, create_command_pool,
            create_debug_callback, create_device_and_queue, create_entry, create_gizmo_ubo,
            create_id_buffers, create_instance, create_primitive_id_buffer,
            create_render_command_buffers, get_display_handle, get_window_handle,
            shaders_should_write_linear_color,
        },
    },
    user_interface::{
        camera::Camera,
        gizmo::{GizmoElement, GizmoVisibility},
    },
};
use anyhow::Context;
use ash::vk;
use bort_vk::{
    AllocationAccess, Buffer, CommandBuffer, CommandPool, DebugCallback, Device, Fence,
    Framebuffer, Image, ImageView, Instance, MemoryAllocator, Queue, RenderPass, Semaphore,
    Surface, Swapchain, SwapchainImage,
};
use egui::{ClippedPrimitive, TexturesDelta};
use glam::Vec3;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::mem;
use std::sync::Arc;
use winit::window::Window;

/// Contains Vulkan resources and methods to manage rendering
pub struct RenderManager {
    _instance: Arc<Instance>,
    _debug_callback: Option<Arc<DebugCallback>>,
    device: Arc<Device>,

    render_queue: Arc<Queue>,
    transfer_queue: Arc<Queue>,

    memory_allocator: Arc<MemoryAllocator>,
    _command_pool_render: Arc<CommandPool>,
    _command_pool_transfer: Arc<CommandPool>,

    window: Arc<Window>,
    surface: Arc<Surface>,
    swapchain: Arc<Swapchain>,
    /// Per swapchain image
    swapchain_image_views: Vec<Arc<ImageView<SwapchainImage>>>,
    shaders_write_linear_color: bool,

    render_pass: Arc<RenderPass>,
    /// Outer vec: per swapchain image; Inner vec: per FRAMES_IN_FLIGHT
    framebuffers: Vec<Vec<Framebuffer>>,
    /// One for each framebuffer attachment
    clear_values: Vec<vk::ClearValue>,

    depth_buffer: Arc<ImageView<Image>>,
    normal_buffer: Arc<ImageView<Image>>,
    albedo_buffer: Arc<ImageView<Image>>,
    primitive_id_buffer: Arc<ImageView<Image>>,
    /// Per FRAMES_IN_FLIGHT
    id_buffers: Vec<Arc<ImageView<Image>>>,
    camera_buffer: Buffer,
    gizmo_buffer: Buffer,

    // render passes
    geometry_pass: GeometryPass,
    lighting_pass: LightingPass,
    gizmo_pass: GizmoPass,
    overlay_pass: OverlayPass,
    gui_pass: GuiPass,

    object_id_reader: ElementIdReader,

    /// Per FRAMES_IN_FLIGHT
    render_command_buffers: Vec<CommandBuffer>,
    /// Per FRAMES_IN_FLIGHT
    render_fences: Vec<Fence>,
    /// Per FRAMES_IN_FLIGHT
    semaphores_swapchain_image_available: Vec<Semaphore>,
    /// Per swapchain image
    semaphores_present_swapchain_image: Vec<Semaphore>,

    renderer_state: RendererState,
    /// Indicates which framebuffer is being processed right now.
    frame_index_currently_rendering: usize,
    /// Can be set to true with [`Self::set_window_just_resized_flag`] and set to false in [`Self::render_frame`]
    window_just_resized: bool,
}

// Public functions

impl RenderManager {
    /// Initializes Vulkan resources. If renderer fails to initiver_minoralize, returns a string explanation.
    pub fn new(window: Arc<Window>, scale_factor: f32) -> anyhow::Result<Self> {
        let entry = create_entry()?;

        let display_handle = get_display_handle(&window)?;
        let window_handle = get_window_handle(&window)?;

        let instance = create_instance(entry.clone(), &display_handle)?;
        let debug_callback = create_debug_callback(&instance);

        let surface = Arc::new(
            Surface::new(
                &entry,
                instance.clone(),
                display_handle.as_raw(),
                window_handle.as_raw(),
            )
            .context("creating vulkan surface")?,
        );

        let ChoosePhysicalDeviceReturn {
            physical_device,
            render_queue_family_index,
            transfer_queue_family_index,
        } = choose_physical_device_and_queue_families(instance.clone(), &surface)?;
        let physical_device = Arc::new(physical_device);

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

        let command_pool_render = create_command_pool(device.clone(), &render_queue)?;
        let command_pool_transfer = create_command_pool(device.clone(), &transfer_queue)?;

        let memory_allocator = Arc::new(MemoryAllocator::new(device.clone())?);

        let swapchain = create_swapchain(device.clone(), surface.clone(), &window)?;
        let shaders_write_linear_color =
            shaders_should_write_linear_color(swapchain.properties().surface_format);

        let swapchain_image_views = create_swapchain_image_views(&swapchain)?;
        let swapchain_len = swapchain_image_views.len();
        debug!("swapchain image count = {}", swapchain_len);

        let depth_buffer_format = choose_depth_buffer_format(&physical_device)?;

        let render_pass =
            create_render_pass(device.clone(), swapchain.properties(), depth_buffer_format)?;

        let framebuffer_dimensions = swapchain.properties().dimensions();
        let depth_buffer = create_depth_buffer(
            memory_allocator.clone(),
            framebuffer_dimensions,
            depth_buffer_format,
        )?;
        let normal_buffer = create_normal_buffer(memory_allocator.clone(), framebuffer_dimensions)?;
        let albedo_buffer = create_albedo_buffer(memory_allocator.clone(), framebuffer_dimensions)?;
        let primitive_id_buffer =
            create_primitive_id_buffer(memory_allocator.clone(), framebuffer_dimensions)?;
        let id_buffers = create_id_buffers(memory_allocator.clone(), framebuffer_dimensions)?;

        let camera_buffer = create_camera_ubo(memory_allocator.clone())?;
        let gizmo_buffer = create_gizmo_ubo(memory_allocator.clone())?;

        let framebuffers = create_framebuffers(
            render_pass.clone(),
            &swapchain_image_views,
            normal_buffer.clone(),
            albedo_buffer.clone(),
            primitive_id_buffer.clone(),
            depth_buffer.clone(),
            &id_buffers,
        )?;

        let clear_values = create_clear_values();

        let geometry_pass = GeometryPass::new(
            device.clone(),
            memory_allocator.clone(),
            &render_pass,
            &camera_buffer,
            transfer_queue_family_index,
            render_queue_family_index,
        )?;
        let lighting_pass = LightingPass::new(
            device.clone(),
            &render_pass,
            &camera_buffer,
            &normal_buffer,
            &albedo_buffer,
            &primitive_id_buffer,
        )?;
        let gizmo_pass = GizmoPass::new(
            memory_allocator.clone(),
            &render_pass,
            &camera_buffer,
            &gizmo_buffer,
        )?;
        let overlay_pass = OverlayPass::new(&render_pass, &camera_buffer)?;
        let gui_pass = GuiPass::new(
            memory_allocator.clone(),
            &render_pass,
            command_pool_render.clone(),
            command_pool_transfer.clone(),
            scale_factor,
        )?;

        let render_command_buffers = create_render_command_buffers(command_pool_render.clone())?;

        let mut render_fences: Vec<Fence> = Vec::with_capacity(FRAMES_IN_FLIGHT);
        let mut semaphores_swapchain_image_available: Vec<Semaphore> =
            Vec::with_capacity(FRAMES_IN_FLIGHT);
        for _i in 0..FRAMES_IN_FLIGHT {
            render_fences.push(Fence::new_signalled(device.clone()).context("creating fence")?);
            semaphores_swapchain_image_available
                .push(Semaphore::new(device.clone()).context("creating semaphore")?);
        }

        let mut semaphores_present_swapchain_image: Vec<Semaphore> =
            Vec::with_capacity(swapchain_len);
        for _i in 0..swapchain_len {
            semaphores_present_swapchain_image
                .push(Semaphore::new(device.clone()).context("creating semaphore")?);
        }

        let object_id_reader = ElementIdReader::new(
            transfer_queue.clone(),
            render_queue.clone(),
            &command_pool_transfer,
            &command_pool_render,
            memory_allocator.clone(),
        )?;

        Ok(Self {
            _instance: instance,
            _debug_callback: debug_callback,
            device,

            render_queue,
            transfer_queue,

            memory_allocator,
            _command_pool_render: command_pool_render,
            _command_pool_transfer: command_pool_transfer,

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
            albedo_buffer,
            primitive_id_buffer,
            id_buffers,
            camera_buffer,
            gizmo_buffer,

            geometry_pass,
            lighting_pass,
            gizmo_pass,
            overlay_pass,
            gui_pass,

            object_id_reader,

            render_command_buffers,
            render_fences,
            semaphores_swapchain_image_available,
            semaphores_present_swapchain_image,

            renderer_state: RendererState::Initialized,
            frame_index_currently_rendering: 0,
            window_just_resized: false,
        })
    }

    /// Warning: doesn't synchronize with any previously submitted render commands
    pub fn init_camera(&mut self, camera: &Camera) -> anyhow::Result<()> {
        for i in 0..FRAMES_IN_FLIGHT {
            self.update_camera(camera, i)?;
        }
        Ok(())
    }

    pub fn update_gizmo_center(&mut self, selected_object_center: Vec3) -> anyhow::Result<()> {
        // todo fence
        //self.wait_for_previous_frame_fence()?; // throws up semaphore validation errors?
        self.wait_idle_device()?;

        let write_data = GizmoUniformBuffer::new(selected_object_center, config::GIZMO_SCALE);
        self.gizmo_buffer
            .write_struct(write_data, 0)
            .context("uploading selected object center to gizmo rendering buffer")?;
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
        let next_frame = (self.frame_index_currently_rendering + 1) % FRAMES_IN_FLIGHT;
        self.wait_for_previous_frame_fence(next_frame)?;

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
    pub fn render_frame(
        &mut self,
        debug_options: RenderDebugOptions,
        camera: &Camera,
        gizmo_visibility: GizmoVisibility,
        hovered_gizmo: Option<GizmoElement>,
    ) -> anyhow::Result<()> {
        let new_frame_index = (self.frame_index_currently_rendering + 1) % FRAMES_IN_FLIGHT;

        // wait for previous frame render/resource upload to finish
        self.wait_for_previous_frame_fence(new_frame_index)?;

        self.gui_pass
            .free_previous_vertex_and_index_buffers(new_frame_index);

        // note: I found that this check is needed on wayland because the later commands weren't returning 'out of date'...
        if self.window_just_resized {
            self.window_just_resized = false;
            self.recreate_swapchain()?;
        }

        // aquire next swapchain image
        let aquire_res = self.swapchain.aquire_next_image(
            TIMEOUT_NANOSECS,
            Some(&self.semaphores_swapchain_image_available[new_frame_index]),
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

        self.update_camera(camera, new_frame_index)?;

        self.record_render_commands(
            new_frame_index,
            swapchain_index,
            debug_options,
            gizmo_visibility,
            hovered_gizmo,
        )?;

        self.render_fences[new_frame_index]
            .reset()
            .context("reseting previous render fence")?;

        let submit_command_buffers = [self.render_command_buffers[new_frame_index].handle()];

        let wait_semaphores = [self.semaphores_swapchain_image_available[new_frame_index].handle()];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];

        let present_semaphores =
            [self.semaphores_present_swapchain_image[swapchain_index].handle()];

        let submit_info = vk::SubmitInfo::default()
            .command_buffers(&submit_command_buffers)
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .signal_semaphores(&present_semaphores);

        self.render_queue
            .submit(&[submit_info], Some(&self.render_fences[new_frame_index]))
            .context("submitting render commands")?;
        self.frame_index_currently_rendering = new_frame_index;

        // submit present instruction

        let swapchain_present_indices = [swapchain_index as u32];
        let swapchain_handles = [self.swapchain.handle()];
        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&present_semaphores)
            .image_indices(&swapchain_present_indices)
            .swapchains(&swapchain_handles);

        let present_res = self
            .swapchain
            .queue_present(&self.render_queue, &present_info);

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

        self.renderer_state = RendererState::Rendering;
        Ok(())
    }

    pub fn get_element_at_screen_coordinate(
        &mut self,
        screen_coordinate: [f32; 2],
    ) -> anyhow::Result<Option<ElementAtPoint>> {
        let framebuffer_dimensions = self.swapchain.properties().dimensions();
        if screen_coordinate[0] > framebuffer_dimensions.width() as f32
            || screen_coordinate[1] > framebuffer_dimensions.height() as f32
        {
            return Ok(None);
        }

        if let RendererState::Initialized = self.renderer_state {
            // buffer data and sync is undefined as no render commands have been submitted yet
            warn!("element at screen coordinate queried but renderer is in {:?} state. ignoring request.", self.renderer_state);
            return Ok(None);
        }

        let previous_frame =
            (self.frame_index_currently_rendering + FRAMES_IN_FLIGHT - 1) % FRAMES_IN_FLIGHT;
        let last_id_buffer = self.id_buffers[previous_frame].clone();

        let different_queue_family_indices =
            self.render_queue.family_index() != self.transfer_queue.family_index();

        if different_queue_family_indices {
            // render queue release operation
            self.object_id_reader
                .record_and_submit_pre_transfer_sync_commands(last_id_buffer.clone())?;
        }

        self.object_id_reader
            .record_primitive_id_copy_commands(screen_coordinate, last_id_buffer.clone())?;
        self.object_id_reader.submit_primitive_id_copy_commands()?;

        if different_queue_family_indices {
            // render queue release operation
            let next_frame_index = (self.frame_index_currently_rendering + 1) % FRAMES_IN_FLIGHT;
            self.object_id_reader
                .record_and_submit_post_transfer_sync_commands(last_id_buffer, next_frame_index)?;
        }

        let element_at_point = self.object_id_reader.read_object_id_from_buffer()?;
        Ok(Some(element_at_point))
    }

    pub fn wait_idle_device(&self) -> anyhow::Result<()> {
        self.device.wait_idle().context("calling vkDeviceWaitIdle")
    }

    pub fn max_2d_image_size(&self) -> usize {
        self.device
            .physical_device()
            .limits()
            .max_image_dimension2_d as usize
    }
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
        let swapchain_len = self.swapchain_image_views.len();
        trace!("swapchain image count: {}", swapchain_len);

        let depth_buffer_format = self.depth_buffer.image().properties().format;

        self.render_pass = create_render_pass(
            self.device.clone(),
            self.swapchain.properties(),
            depth_buffer_format,
        )?;

        let framebuffer_dimensions = self.swapchain.properties().dimensions();
        self.normal_buffer =
            create_normal_buffer(self.memory_allocator.clone(), framebuffer_dimensions)?;
        self.albedo_buffer =
            create_albedo_buffer(self.memory_allocator.clone(), framebuffer_dimensions)?;
        self.primitive_id_buffer =
            create_primitive_id_buffer(self.memory_allocator.clone(), framebuffer_dimensions)?;
        self.id_buffers = create_id_buffers(self.memory_allocator.clone(), framebuffer_dimensions)?;
        self.depth_buffer = create_depth_buffer(
            self.memory_allocator.clone(),
            framebuffer_dimensions,
            depth_buffer_format,
        )?;

        self.framebuffers = create_framebuffers(
            self.render_pass.clone(),
            &self.swapchain_image_views,
            self.normal_buffer.clone(),
            self.albedo_buffer.clone(),
            self.primitive_id_buffer.clone(),
            self.depth_buffer.clone(),
            &self.id_buffers,
        )?;

        self.lighting_pass.update_g_buffer(
            &self.normal_buffer,
            &self.albedo_buffer,
            &self.primitive_id_buffer,
        )?;

        Ok(())
    }

    fn update_camera(&mut self, camera: &Camera, frame_index: usize) -> anyhow::Result<()> {
        let dimensions = self.swapchain.properties().width_height;
        let camera_data = CameraUniformBuffer::from_camera(
            camera,
            [dimensions[0] as f32, dimensions[1] as f32],
            self.shaders_write_linear_color,
        );

        let offset: usize = mem::size_of::<CameraUniformBuffer>() * frame_index;
        self.camera_buffer
            .write_struct(camera_data, offset)
            .context("uploading camera ubo data")?;

        Ok(())
    }

    fn reset_render_command_buffers(&self) -> anyhow::Result<()> {
        self.render_queue
            .wait_idle()
            .context("calling vkQueueWaitIdle for render queue")?;

        for command_buffer in &self.render_command_buffers {
            command_buffer
                .reset(vk::CommandBufferResetFlags::empty())
                .context("resetting render command buffers")?;
        }
        Ok(())
    }

    fn wait_for_previous_frame_fence(&mut self, frame_index: usize) -> anyhow::Result<()> {
        let fence_wait_res = self.render_fences[frame_index].wait(TIMEOUT_NANOSECS);

        if let Err(fence_wait_err) = fence_wait_res {
            if fence_wait_err == vk::Result::TIMEOUT {
                error!(
                    "previous render fence timed out! timeout set to {}ns",
                    TIMEOUT_NANOSECS
                );
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
        frame_index: usize,
        swapchain_index: usize,
        debug_options: RenderDebugOptions,
        gizmo_visibility: GizmoVisibility,
        hovered_gizmo: Option<GizmoElement>,
    ) -> anyhow::Result<()> {
        let viewport = self.framebuffers[swapchain_index][frame_index].whole_viewport();
        let render_area = self.framebuffers[swapchain_index][frame_index].whole_rect();
        let command_buffer = &self.render_command_buffers[frame_index];

        let begin_info = vk::CommandBufferBeginInfo {
            flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            ..Default::default()
        };
        command_buffer
            .begin(&begin_info)
            .context("beinning render command buffer recording")?;

        let render_pass_begin = vk::RenderPassBeginInfo::default()
            .render_pass(self.render_pass.handle())
            .framebuffer(self.framebuffers[swapchain_index][frame_index].handle())
            .render_area(render_area)
            .clear_values(self.clear_values.as_slice());
        command_buffer.begin_render_pass(&render_pass_begin, vk::SubpassContents::INLINE);

        self.geometry_pass
            .record_commands(command_buffer, frame_index, viewport, render_area);

        command_buffer.next_subpass(vk::SubpassContents::INLINE);

        self.lighting_pass
            .record_commands(command_buffer, frame_index, viewport, render_area);

        if debug_options.enable_aabb_wire_display {
            self.overlay_pass.record_aabb_overlay_commands(
                command_buffer,
                frame_index,
                self.geometry_pass.object_buffer_manager(),
                viewport,
                render_area,
            );
        }

        if gizmo_visibility.any_visible() {
            self.gizmo_pass.record_commands(
                command_buffer,
                frame_index,
                viewport,
                render_area,
                gizmo_visibility,
                hovered_gizmo,
            );
        }

        self.gui_pass.record_render_commands(
            command_buffer,
            frame_index,
            self.shaders_write_linear_color,
            [viewport.width, viewport.height],
        )?;

        command_buffer.end_render_pass();

        command_buffer
            .end()
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
    }
}

// ~~ Helper structs ~~

#[derive(Debug, Clone, Copy)]
enum RendererState {
    /// Before rendering has started. Undefined rendering data.
    Initialized,
    /// Rendering commands have been submitted since initialization.
    Rendering,
}
