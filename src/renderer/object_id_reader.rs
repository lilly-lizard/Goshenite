use super::{config_renderer::TIMEOUT_NANOSECS, vulkan_init::create_cpu_read_staging_buffer};
use crate::engine::object::{object::ObjectId, primitive_op::PrimitiveOpId};
use anyhow::Context;
use ash::vk;
use bort_vk::{
    Buffer, CommandBuffer, CommandPool, Device, DeviceOwned, Fence, Image, ImageAccess, ImageView,
    MemoryAllocator, Queue, Semaphore,
};
use std::sync::Arc;

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
                let object_id_u32 = encoded_id >> 16;
                let object_id = ObjectId::from(object_id_u32 as u16);

                let primitive_op_id_u32 = encoded_id & 0x0000FFFF;
                let primitive_op_id = PrimitiveOpId::from(primitive_op_id_u32 as u16);

                Self::Object {
                    object_id,
                    primitive_op_id,
                }
            }
        }
    }
}

pub(super) struct ObjectIdReader {
    pub device: Arc<Device>,
    pub transfer_queue: Arc<Queue>,
    pub render_queue: Arc<Queue>,

    pub command_buffer_transfer: Arc<CommandBuffer>,
    pub command_buffer_post_render_sync: Arc<CommandBuffer>,
    pub command_buffer_pre_render_sync: Arc<CommandBuffer>,

    pub completion_fence: Arc<Fence>,
    pub semaphore_before_transfer: Arc<Semaphore>,
    pub semaphore_after_transfer: Arc<Semaphore>,

    pub cpu_read_staging_buffer: Buffer,
}

impl ObjectIdReader {
    pub fn new(
        transfer_queue: Arc<Queue>,
        render_queue: Arc<Queue>,
        command_pool_transfer_queue: &Arc<CommandPool>,
        command_pool_render_queue: &Arc<CommandPool>,
        memory_allocator: Arc<MemoryAllocator>,
    ) -> anyhow::Result<Self> {
        let device = transfer_queue.device().clone();

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
            device,
            transfer_queue,
            render_queue,

            command_buffer_transfer,
            command_buffer_post_render_sync,
            command_buffer_pre_render_sync,

            completion_fence,
            semaphore_before_transfer,
            semaphore_after_transfer,

            cpu_read_staging_buffer,
        })
    }

    pub fn record_and_submit_pre_transfer_sync_commands(
        &self,
        last_primitive_id_buffer: Arc<ImageView<Image>>,
    ) -> anyhow::Result<()> {
        let device_ash = self.device.inner();

        let semaphores_before_transfer = [self.semaphore_before_transfer.handle()];

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

        let command_buffer_render_sync = self.command_buffer_post_render_sync.clone();

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

    pub fn record_primitive_id_copy_commands(
        &self,
        screen_coordinate: [f32; 2],
        last_primitive_id_buffer: Arc<ImageView<Image>>,
    ) -> anyhow::Result<()> {
        let different_queue_family_indices =
            self.render_queue.family_index() != self.transfer_queue.family_index();

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
        self.command_buffer_transfer
            .begin(&begin_info)
            .context("beginning command buffer get_element_at_screen_coordinate")?;

        let device_ash = self.device.inner();
        unsafe {
            let command_buffer_handle = self.command_buffer_transfer.handle();

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
                self.cpu_read_staging_buffer.handle(),
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

        self.command_buffer_transfer
            .end()
            .context("ending command buffer get_element_at_screen_coordinate")?;

        Ok(())
    }

    pub fn submit_primitive_id_copy_commands(&self) -> anyhow::Result<()> {
        let semaphores_before_transfer = [self.semaphore_before_transfer.handle()];
        let semaphores_after_transfer = [self.semaphore_after_transfer.handle()];

        let transfer_submit_command_buffers = [self.command_buffer_transfer.handle()];

        let mut transfer_submit_info =
            vk::SubmitInfo::builder().command_buffers(&transfer_submit_command_buffers);

        let different_queue_family_indices =
            self.render_queue.family_index() != self.transfer_queue.family_index();
        if different_queue_family_indices {
            transfer_submit_info = transfer_submit_info
                .wait_semaphores(&semaphores_before_transfer)
                .wait_dst_stage_mask(&[vk::PipelineStageFlags::TRANSFER])
                .signal_semaphores(&semaphores_after_transfer);
        }

        self.completion_fence
            .reset()
            .context("resetting primitive id buffer reset fn")?;

        unsafe {
            self.device
                .inner()
                .queue_submit(
                    self.transfer_queue.handle(),
                    &[transfer_submit_info.build()],
                    self.completion_fence.handle(),
                )
                .context("submitting commands to read primitive id at coordinate")?;
        }

        Ok(())
    }

    pub fn record_and_submit_post_transfer_sync_commands(
        &self,
        last_primitive_id_buffer: Arc<ImageView<Image>>,
    ) -> anyhow::Result<()> {
        let device_ash = self.device.inner();

        let semaphores_after_transfer = [self.semaphore_before_transfer.handle()];

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

        let command_buffer_render_sync = self.command_buffer_pre_render_sync.clone();

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

    pub fn read_object_id_from_buffer(&mut self) -> anyhow::Result<ElementAtPoint> {
        self.completion_fence
            .wait(TIMEOUT_NANOSECS)
            .context("waiting for render id buffer copy fence")?;

        let rendered_id = self
            .cpu_read_staging_buffer
            .memory_allocation_mut()
            .read_struct::<u32>(0)
            .context("reading render id")?;

        Ok(ElementAtPoint::from_rendered_id(rendered_id))
    }
}
