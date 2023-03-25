use super::{
    config_renderer::{ENABLE_VULKAN_VALIDATION, TIMEOUT_NANOSECS, VULKAN_VER_MAJ, VULKAN_VER_MIN},
    geometry_pass::GeometryPass,
    gui_pass::GuiPass,
    lighting_pass::LightingPass,
    shader_interfaces::uniform_buffers::CameraUniformBuffer,
    vulkan_init::{
        choose_physical_device_and_queue_families, create_camera_ubo, create_clear_values,
        create_depth_buffer, create_device_and_queues, create_framebuffers, create_normal_buffer,
        create_per_frame_fence, create_primitive_id_buffer, create_render_pass, create_swapchain,
        create_swapchain_image_views, ChoosePhysicalDeviceReturn, CreateDeviceAndQueuesReturn,
    },
};
use crate::{
    config::ENGINE_NAME,
    engine::object::{object_collection::ObjectCollection, objects_delta::ObjectsDelta},
    renderer::vulkan_init::{create_command_pool, create_render_command_buffers},
    user_interface::{camera::Camera, gui::Gui},
};
use anyhow::Context;
use ash::{vk, Entry};
use bort::{
    is_format_srgb, ApiVersion, Buffer, CommandBuffer, CommandPool, DebugCallback, Device, Fence,
    Framebuffer, Image, ImageView, Instance, MemoryAllocator, Queue, RenderPass, Semaphore,
    Surface, Swapchain, SwapchainImage,
};
use egui::TexturesDelta;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::{borrow::Cow, ffi::CStr, sync::Arc};
use winit::window::Window;

/// Contains Vulkan resources and methods to manage rendering
pub struct RenderManager {
    instance: Arc<Instance>,
    debug_callback: Option<DebugCallback>,

    device: Arc<Device>,
    render_queue: Queue,

    memory_allocator: Arc<MemoryAllocator>,
    command_pool: Arc<CommandPool>,

    window: Arc<Window>,
    surface: Arc<Surface>,
    swapchain: Swapchain,
    swapchain_image_views: Vec<Arc<ImageView<SwapchainImage>>>,
    is_swapchain_srgb: bool,

    render_pass: Arc<RenderPass>,
    framebuffers: Vec<Arc<Framebuffer>>,
    clear_values: Vec<vk::ClearValue>,

    depth_buffer: Arc<ImageView<Image>>,
    normal_buffer: Arc<ImageView<Image>>,
    primitive_id_buffer: Arc<ImageView<Image>>,
    camera_ubo: Arc<Buffer>,

    lighting_pass: LightingPass,
    geometry_pass: GeometryPass,
    gui_pass: GuiPass,

    render_command_buffers: Vec<Arc<CommandBuffer>>,
    previous_render_fence: Arc<Fence>, // per frame in flight
    previous_upload_fence: Option<Arc<Fence>>, // just one
    next_frame_wait_semaphore: Arc<Semaphore>, // per frame in flight
    swapchain_image_available_semaphore: Arc<Semaphore>, // per frame in flight

    /// Some resources are duplicated `FRAMES_IN_FLIGHT` times in order to manipulate resources
    /// without conflicting with commands currently being processed. This variable indicates
    /// which index to will be next submitted to the GPU.
    next_frame: usize,
}

// Public functions

impl RenderManager {
    /// Initializes Vulkan resources. If renderer fails to initiver_minoralize, returns a string explanation.
    pub fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let entry = unsafe { Entry::load() }
            .context("loading vulkan dynamic library. please install vulkan on your system...")?;
        let entry = Arc::new(entry);

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
        let debug_callback = if ENABLE_VULKAN_VALIDATION {
            match DebugCallback::new(&entry, instance.clone(), Some(log_vulkan_debug_callback)) {
                Ok(x) => {
                    info!("enabling vulkan validation layers and debug callback");
                    Some(x)
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
        } = create_device_and_queues(
            physical_device.clone(),
            render_queue_family_index,
            transfer_queue_family_index,
        )?;

        let memory_allocator = Arc::new(MemoryAllocator::new(device.clone())?);

        let command_pool = create_command_pool(device.clone(), &render_queue)?;

        let swapchain =
            create_swapchain(device.clone(), surface.clone(), &window, &physical_device)?;
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
        let is_swapchain_srgb = is_format_srgb(swapchain.properties().surface_format.format);

        let swapchain_image_views = create_swapchain_image_views(&swapchain)?;

        let render_pass = create_render_pass(device.clone(), swapchain.properties())?;

        let depth_buffer = create_depth_buffer(
            memory_allocator.clone(),
            swapchain.properties().dimensions(),
        )?;

        let normal_buffer = create_normal_buffer(
            memory_allocator.clone(),
            swapchain.properties().dimensions(),
        )?;

        let primitive_id_buffer = create_primitive_id_buffer(
            memory_allocator.clone(),
            swapchain.properties().dimensions(),
        )?;

        let camera_ubo = create_camera_ubo(memory_allocator.clone())?;

        let framebuffers = create_framebuffers(
            &render_pass,
            &swapchain_image_views,
            &normal_buffer,
            &primitive_id_buffer,
            &depth_buffer,
        )?;

        let clear_values = create_clear_values();

        let geometry_pass = GeometryPass::new(
            device.clone(),
            memory_allocator.clone(),
            &render_pass,
            &camera_ubo,
        )?;

        let lighting_pass = LightingPass::new(
            device.clone(),
            &render_pass,
            &camera_ubo,
            normal_buffer.as_ref(),
            primitive_id_buffer.as_ref(),
        )?;

        let gui_pass = GuiPass::new(
            device.clone(),
            memory_allocator.clone(),
            &render_pass,
            render_queue_family_index,
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
            is_swapchain_srgb,

            render_pass,
            framebuffers,
            clear_values,

            depth_buffer,
            normal_buffer,
            primitive_id_buffer,
            camera_ubo,

            geometry_pass,
            lighting_pass,
            gui_pass,

            render_command_buffers,
            previous_render_fence,
            previous_upload_fence: None,
            next_frame_wait_semaphore,
            swapchain_image_available_semaphore,

            next_frame: 0,
        })
    }

    pub fn update_camera(&mut self, camera: &mut Camera) -> anyhow::Result<()> {
        self.wait_idle()?;

        let dimensions = self.swapchain.properties().width_height;
        let camera_data =
            CameraUniformBuffer::from_camera(camera, [dimensions[0] as f32, dimensions[1] as f32]);

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
        object_delta: ObjectsDelta,
    ) -> anyhow::Result<()> {
        self.geometry_pass
            .update_object_buffers(object_collection, object_delta)
    }

    pub fn update_gui_textures(
        &mut self,
        textures_delta_vec: Vec<TexturesDelta>,
    ) -> anyhow::Result<()> {
        self.wait_for_fences()?;

        let texture_update_fence_option = self
            .gui_pass
            .update_textures(textures_delta_vec, &self.render_queue)?;

        if let Some(previous_upload_fence) = texture_update_fence_option {
            self.previous_upload_fence = Some(previous_upload_fence);
        }

        Ok(())
    }

    /// Submits Vulkan commands for rendering a frame.
    pub fn render_frame(&mut self, window_resize: bool, gui: &mut Gui) -> anyhow::Result<()> {
        // wait for previous frame render/resource upload to finish

        self.wait_for_fences()?;

        self.gui_pass.free_previous_vertex_and_index_buffers();

        if window_resize {
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
                return self.recreate_swapchain();
            } else {
                return Err(aquire_err).context("calling vkAcquireNextImageKHR");
            }
        }

        let (swapchain_index, swapchain_is_suboptimal) =
            aquire_res.expect("handled err case in previous lines");
        let swapchain_index = swapchain_index as usize;
        if swapchain_is_suboptimal {
            return self.recreate_swapchain();
        }

        // record commands

        // todo sub-command

        let command_buffer = self.render_command_buffers[swapchain_index].clone();
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
            gui,
            self.is_swapchain_srgb,
            [viewport.width, viewport.height],
        )?;

        unsafe { device_ash.cmd_end_render_pass(command_buffer_handle) };

        unsafe { device_ash.end_command_buffer(command_buffer_handle) }
            .context("ending render command buffer recording")?;

        // submit commands

        let previous_render_fence_handle = self.previous_render_fence.handle();
        unsafe { device_ash.reset_fences(&[previous_render_fence_handle]) }
            .context("reseting previous render fence")?;

        let submit_command_buffers = [command_buffer_handle];

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
                self.recreate_swapchain()?;
            } else {
                return Err(present_err).context("submitting swapchain present instruction")?;
            }
        }

        Ok(())
    }
}

// Private functions

impl RenderManager {
    fn wait_idle(&mut self) -> anyhow::Result<()> {
        self.device.wait_idle().context("calling vkDeviceWaitIdle")
    }

    /// Recreates the swapchain, g-buffers and assiciated descriptor sets, then unsets `recreate_swapchain` trigger.
    fn recreate_swapchain(&mut self) -> anyhow::Result<()> {
        // clean up resources depending on the swapchain
        self.framebuffers.clear();
        self.swapchain_image_views.clear();

        // recreate the swapchain
        self.swapchain.recreate().context("recreating swapchain")?;

        // reinitialize related resources
        self.is_swapchain_srgb = is_format_srgb(self.swapchain.properties().surface_format.format);
        self.swapchain_image_views = create_swapchain_image_views(&self.swapchain)?;

        self.render_pass = create_render_pass(self.device.clone(), self.swapchain.properties())?;

        self.normal_buffer = create_normal_buffer(
            self.memory_allocator.clone(),
            self.swapchain.properties().dimensions(),
        )?;

        self.primitive_id_buffer = create_primitive_id_buffer(
            self.memory_allocator.clone(),
            self.swapchain.properties().dimensions(),
        )?;

        self.depth_buffer = create_depth_buffer(
            self.memory_allocator.clone(),
            self.swapchain.properties().dimensions(),
        )?;

        self.framebuffers = create_framebuffers(
            &self.render_pass,
            &self.swapchain_image_views,
            &self.normal_buffer,
            &self.primitive_id_buffer,
            &self.depth_buffer,
        )?;

        self.lighting_pass
            .update_g_buffers(&self.normal_buffer, &self.primitive_id_buffer)?;

        Ok(())
    }

    fn wait_for_fences(&mut self) -> anyhow::Result<()> {
        let mut wait_fence_handles = vec![self.previous_render_fence.handle()];
        if let Some(previous_upload_fence) = &self.previous_upload_fence {
            wait_fence_handles.push(previous_upload_fence.handle());
        }

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

        self.previous_upload_fence = None;

        Ok(())
    }
}

impl Drop for RenderManager {
    fn drop(&mut self) {
        debug!("dropping render manager");
    }
}

unsafe extern "system" fn log_vulkan_debug_callback(
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

    match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            error!("Vulkan [{:?}]:\n{}", message_type, message);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            warn!("Vulkan [{:?}]:\n{}", message_type, message);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => {
            info!("Vulkan [{:?}]:\n{}", message_type, message);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => {
            trace!("Vulkan [{:?}]:\n{}", message_type, message);
        }
        _ => trace!(
            "Vulkan [{:?}] (UNKONWN SEVERITY):\n{}",
            message_type,
            message
        ),
    }

    vk::FALSE
}
