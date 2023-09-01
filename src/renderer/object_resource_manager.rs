use super::{
    geometry_pass::descriptor,
    shader_interfaces::{
        primitive_op_buffer::{PrimitiveOpBufferUnit, PRIMITIVE_PACKET_LEN},
        vertex_inputs::BoundingBoxVertex,
    },
};
use crate::engine::{
    aabb::AABB_VERTEX_COUNT,
    object::object::{ObjectDuplicate, ObjectId},
};
use anyhow::Context;
use ash::vk;
use bort_vk::{
    allocation_info_cpu_accessible, allocation_info_from_flags, AllocAccess, Buffer,
    BufferProperties, CommandBuffer, CommandPool, CommandPoolProperties, DescriptorPool,
    DescriptorPoolProperties, DescriptorSet, DescriptorSetLayout, Device, DeviceOwned,
    GraphicsPipeline, MemoryAllocator, PipelineAccess, Queue,
};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::{mem::size_of, sync::Arc};

const DESCRIPTOR_POOL_SIZE: u32 = 256;

// TODO biggest optimization is a staging buffer to make the vertex/storage buffers a more optimized memory type

/// Reserve for 1024 primitive ops
const PRIMITIVE_OPS_STAGING_BUFFER_SIZE: vk::DeviceSize =
    (1024 * PRIMITIVE_PACKET_LEN * size_of::<PrimitiveOpBufferUnit>()) as vk::DeviceSize;

/// Reserve for 16 AABBs (one aabb per object)
const BOUNDING_MESH_STAGING_BUFFER_SIZE: vk::DeviceSize =
    (16 * AABB_VERTEX_COUNT * size_of::<BoundingBoxVertex>()) as vk::DeviceSize;

#[derive(Clone)]
struct PerObjectResources {
    pub object_id: ObjectId,
    pub bounding_mesh_buffer: Arc<Buffer>,
    pub bounding_mesh_vertex_count: u32,
    pub primitive_ops_buffer: Arc<Buffer>,
    pub primitive_ops_descriptor_set: Arc<DescriptorSet>,
}

/// Manages per-object resources for the geometry pass
pub struct ObjectResourceManager {
    device: Arc<Device>,

    memory_allocator: Arc<MemoryAllocator>,
    transient_command_pool: Arc<CommandPool>,

    primitive_ops_staging_buffer: Buffer,
    bounding_mesh_staging_buffer: Buffer,
    primitive_ops_staging_buffer_offset: vk::DeviceSize,
    bounding_mesh_staging_buffer_offset: vk::DeviceSize,

    objects_buffers: Vec<PerObjectResources>,

    descriptor_pools: Vec<Arc<DescriptorPool>>,
    primitive_ops_desc_set_layout: Arc<DescriptorSetLayout>,
}

impl ObjectResourceManager {
    pub fn new(
        memory_allocator: Arc<MemoryAllocator>,
        primitive_ops_desc_set_layout: Arc<DescriptorSetLayout>,
        queue_family_index: u32,
    ) -> anyhow::Result<Self> {
        let device = memory_allocator.device().clone();

        let initial_descriptor_pool = create_descriptor_pool(device.clone())?;
        let transient_command_pool =
            create_transient_command_pool(device.clone(), queue_family_index)?;

        let primitive_ops_staging_buffer =
            create_primitive_ops_staging_buffer(memory_allocator.clone())?;
        let bounding_mesh_staging_buffer =
            create_bounding_mesh_staging_buffer(memory_allocator.clone())?;

        Ok(Self {
            device,

            memory_allocator,
            transient_command_pool,

            primitive_ops_staging_buffer,
            bounding_mesh_staging_buffer,
            primitive_ops_staging_buffer_offset: 0,
            bounding_mesh_staging_buffer_offset: 0,

            objects_buffers: Vec::new(),

            descriptor_pools: vec![initial_descriptor_pool],
            primitive_ops_desc_set_layout,
        })
    }

    pub fn update_or_push(
        &mut self,
        object_id: ObjectId,
        object: ObjectDuplicate,
        queue: &Queue,
    ) -> anyhow::Result<()> {
        // create one-time command buffer
        let command_buffer = CommandBuffer::new(
            self.transient_command_pool.clone(),
            vk::CommandBufferLevel::PRIMARY,
        )
        .context("creating command buffer for geometry upload")?;

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        command_buffer
            .begin(&begin_info)
            .context("beginning geometry upload command buffer")?;

        let primitive_ops_buffer = self
            .upload_primitive_ops(object_id, &object, &command_buffer)
            .context("initial upload object to buffer")?;

        if let Some(index) = self.get_index(object_id) {
            let bounding_mesh_buffer =
                self.upload_bounding_mesh(object_id, &object, &command_buffer)?;

            write_desc_set_primitive_ops(
                &self.objects_buffers[index].primitive_ops_descriptor_set,
                &primitive_ops_buffer,
            )?;

            self.objects_buffers[index].bounding_mesh_buffer = bounding_mesh_buffer;
            self.objects_buffers[index].bounding_mesh_vertex_count = AABB_VERTEX_COUNT as u32;
            self.objects_buffers[index].primitive_ops_buffer = primitive_ops_buffer;
        } else {
            let bounding_mesh_buffer =
                self.upload_bounding_mesh(object_id, &object, &command_buffer)?;

            let primitive_ops_descriptor_set = self.allocate_primitive_ops_descriptor_set()?;
            write_desc_set_primitive_ops(&primitive_ops_descriptor_set, &primitive_ops_buffer)?;

            let new_object = PerObjectResources {
                object_id,
                bounding_mesh_buffer,
                bounding_mesh_vertex_count: AABB_VERTEX_COUNT as u32,
                primitive_ops_buffer,
                primitive_ops_descriptor_set,
            };
            self.objects_buffers.push(new_object);
        }

        command_buffer
            .end()
            .context("ending geometry upload command buffer")?;

        // submit upload commands
        let command_buffer_handles = [command_buffer.handle()];
        let submit_info = vk::SubmitInfo::builder().command_buffers(&command_buffer_handles);
        queue
            .submit(&[*submit_info], None)
            .context("submitting geometry buffer upload commands")?;

        Ok(())
    }

    pub fn draw_commands(
        &self,
        command_buffer: &CommandBuffer,
        pipeline: &GraphicsPipeline,
    ) -> anyhow::Result<()> {
        let device_ash = self.device.inner();
        let command_buffer_handle = command_buffer.handle();

        // for each object
        for per_object_buffers in self.objects_buffers.iter() {
            unsafe {
                device_ash.cmd_bind_descriptor_sets(
                    command_buffer_handle,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.pipeline_layout().handle(),
                    1,
                    &[per_object_buffers.primitive_ops_descriptor_set.handle()],
                    &[],
                );
                device_ash.cmd_bind_vertex_buffers(
                    command_buffer_handle,
                    0,
                    &[per_object_buffers.bounding_mesh_buffer.handle()],
                    &[0],
                );
                device_ash.cmd_draw(
                    command_buffer_handle,
                    per_object_buffers.bounding_mesh_vertex_count,
                    1,
                    0,
                    0,
                );
            }
        }

        Ok(())
    }

    /// Returns the vec index if the id was found and removed.
    pub fn remove(&mut self, id: ObjectId) -> Option<usize> {
        let index_res = self.get_index(id);
        if let Some(index) = index_res {
            self.objects_buffers.remove(index);
        }
        index_res
    }

    pub fn get_index(&self, object_id: ObjectId) -> Option<usize> {
        self.objects_buffers
            .iter()
            .position(|o| o.object_id == object_id)
    }

    pub fn primitive_op_buffers(&self) -> Vec<Arc<Buffer>> {
        self.objects_buffers
            .iter()
            .map(|o| o.primitive_ops_buffer.clone())
            .collect::<Vec<_>>()
    }

    pub fn bounding_mesh_buffers(&self) -> Vec<Arc<Buffer>> {
        self.objects_buffers
            .iter()
            .map(|o| o.bounding_mesh_buffer.clone())
            .collect::<Vec<_>>()
    }

    pub fn object_count(&self) -> usize {
        self.objects_buffers.len()
    }

    /// Resets the offsets for the bounding mesh and primitive ops upload staging buffers.
    /// Make sure this is only called after commands from `Self::update_or_push` are completed
    /// because these offsets are used to ensure that data required by previous upload commands is
    /// being conserved.
    pub fn reset_staging_buffer_offsets(&mut self) {
        self.bounding_mesh_staging_buffer_offset = 0;
        self.primitive_ops_staging_buffer_offset = 0;
    }
}

// Private functions

impl ObjectResourceManager {
    fn allocate_primitive_ops_descriptor_set(&mut self) -> anyhow::Result<Arc<DescriptorSet>> {
        let descriptor_pool = self.descriptor_pools[self.descriptor_pools.len() - 1].clone();

        let alloc_res =
            DescriptorSet::new(descriptor_pool, self.primitive_ops_desc_set_layout.clone());

        let desc_set = match alloc_res {
            Err(alloc_err) => {
                if alloc_err == vk::Result::ERROR_OUT_OF_POOL_MEMORY {
                    // old pool ran out of space -> create a new descriptor pool
                    let new_descriptor_pool = create_descriptor_pool(self.device.clone())?;
                    self.descriptor_pools.push(new_descriptor_pool.clone());

                    DescriptorSet::new(
                        new_descriptor_pool,
                        self.primitive_ops_desc_set_layout.clone(),
                    )
                    .context("allocating primitive ops desc set after creating new pool")?
                } else {
                    return Err(alloc_err).context("allocating primitive ops desc set");
                }
            }
            Ok(desc_set) => desc_set,
        };

        Ok(Arc::new(desc_set))
    }

    fn upload_bounding_mesh(
        &mut self,
        object_id: ObjectId,
        object: &ObjectDuplicate,
        command_buffer: &CommandBuffer,
    ) -> anyhow::Result<Arc<Buffer>> {
        trace!(
            "uploading bounding box vertices for object id = {:?} to gpu buffer",
            object_id
        );

        let data = object.aabb().vertices(object_id);
        let data_size = std::mem::size_of_val(&data) as vk::DeviceSize;

        // create new buffer

        let buffer_props = BufferProperties::new_default(
            data_size,
            vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
        );

        let alloc_info = allocation_info_from_flags(
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            vk::MemoryPropertyFlags::empty(),
        );

        let mut new_buffer = Buffer::new(self.memory_allocator.clone(), buffer_props, alloc_info)
            .context("creating geometry pass bounding box buffer")?;

        // upload data

        let new_buffer_is_host_visible = new_buffer
            .memory_allocation()
            .memory_property_flags()
            .contains(vk::MemoryPropertyFlags::HOST_VISIBLE);

        if new_buffer_is_host_visible {
            // don't bother with staging buffer (unified memory architecture)
            new_buffer
                .write_iter(data, 0)
                .context("uploading geometry pass bounding box vertices to staging buffer")?;
        } else {
            // need staging buffer to access gpu only memory
            self.upload_bounding_mesh_with_staging_buffer(
                data,
                &new_buffer,
                data_size,
                command_buffer,
            )?;
        }
        // more info about this topic here: https://asawicki.info/news_1740_vulkan_memory_types_on_pc_and_how_to_use_them

        Ok(Arc::new(new_buffer))
    }

    fn upload_bounding_mesh_with_staging_buffer<I, T>(
        &mut self,
        data: I,
        new_buffer: &Buffer,
        data_size: u64,
        command_buffer: &CommandBuffer,
    ) -> anyhow::Result<()>
    where
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        self.bounding_mesh_staging_buffer
            .write_iter(data, self.bounding_mesh_staging_buffer_offset as usize)
            .context("uploading geometry pass bounding box vertices to staging buffer")?;

        let after_transfer_barrier = vk::BufferMemoryBarrier::builder()
            .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .dst_access_mask(vk::AccessFlags::VERTEX_ATTRIBUTE_READ)
            .buffer(new_buffer.handle())
            .size(data_size)
            .offset(0)
            .build();

        let copy_region = vk::BufferCopy {
            src_offset: self.bounding_mesh_staging_buffer_offset,
            dst_offset: 0,
            size: data_size,
        };

        unsafe {
            self.device.inner().cmd_copy_buffer(
                command_buffer.handle(),
                self.bounding_mesh_staging_buffer.handle(),
                new_buffer.handle(),
                &[copy_region],
            );

            self.device.inner().cmd_pipeline_barrier(
                command_buffer.handle(),
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::VERTEX_INPUT,
                vk::DependencyFlags::empty(),
                &[],
                &[after_transfer_barrier],
                &[],
            );
        }

        self.bounding_mesh_staging_buffer_offset += data_size;

        Ok(())
    }

    fn upload_primitive_ops(
        &mut self,
        object_id: ObjectId,
        object: &ObjectDuplicate,
        command_buffer: &CommandBuffer,
    ) -> anyhow::Result<Arc<Buffer>> {
        trace!(
            "uploading primitive ops for object id = {:?} to gpu buffer",
            object_id
        );

        let data = object.encoded_primitive_ops(object_id);
        let data_size = std::mem::size_of_val(data.as_slice()) as vk::DeviceSize;

        let buffer_props = BufferProperties::new_default(
            data_size,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
        );

        let alloc_info = allocation_info_from_flags(
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            vk::MemoryPropertyFlags::empty(),
        );

        let new_buffer = Buffer::new(self.memory_allocator.clone(), buffer_props, alloc_info)
            .context("creating geometry pass primitive op buffer")?;

        // upload data

        // todo what do if not enough space? goes for other staging buffers here too...
        self.primitive_ops_staging_buffer
            .write_iter(data, self.primitive_ops_staging_buffer_offset as usize)
            .context("uploading geometry pass primitive ops to staging buffer")?;

        let after_transfer_barrier = vk::BufferMemoryBarrier::builder()
            .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .buffer(new_buffer.handle())
            .size(data_size)
            .offset(0)
            .build();

        let copy_region = vk::BufferCopy {
            src_offset: self.primitive_ops_staging_buffer_offset,
            dst_offset: 0,
            size: data_size,
        };

        unsafe {
            self.device.inner().cmd_copy_buffer(
                command_buffer.handle(),
                self.primitive_ops_staging_buffer.handle(),
                new_buffer.handle(),
                &[copy_region],
            );

            self.device.inner().cmd_pipeline_barrier(
                command_buffer.handle(),
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[after_transfer_barrier],
                &[],
            );
        }

        // so the next primitive op upload doesn't overwrite the data from these primitive ops
        self.primitive_ops_staging_buffer_offset += data_size;

        Ok(Arc::new(new_buffer))
    }
}

fn create_transient_command_pool(
    device: Arc<Device>,
    queue_family_index: u32,
) -> anyhow::Result<Arc<CommandPool>> {
    let command_pool_props = CommandPoolProperties {
        flags: vk::CommandPoolCreateFlags::TRANSIENT,
        queue_family_index,
    };

    let command_pool = CommandPool::new(device, command_pool_props)
        .context("creating gui renderer command pool")?;

    Ok(Arc::new(command_pool))
}

fn create_primitive_ops_staging_buffer(
    memory_allocator: Arc<MemoryAllocator>,
) -> anyhow::Result<Buffer> {
    let buffer_props = BufferProperties::new_default(
        PRIMITIVE_OPS_STAGING_BUFFER_SIZE,
        vk::BufferUsageFlags::TRANSFER_SRC,
    );
    let alloc_info = allocation_info_cpu_accessible();

    Buffer::new(memory_allocator, buffer_props, alloc_info)
        .context("creating primitive op staging buffer")
}

fn create_bounding_mesh_staging_buffer(
    memory_allocator: Arc<MemoryAllocator>,
) -> anyhow::Result<Buffer> {
    let buffer_props = BufferProperties::new_default(
        BOUNDING_MESH_STAGING_BUFFER_SIZE,
        vk::BufferUsageFlags::TRANSFER_SRC,
    );
    let alloc_info = allocation_info_cpu_accessible();

    Buffer::new(memory_allocator, buffer_props, alloc_info)
        .context("creating bounding mesh staging buffer")
}

fn create_descriptor_pool(device: Arc<Device>) -> anyhow::Result<Arc<DescriptorPool>> {
    let descriptor_pool_props = DescriptorPoolProperties {
        max_sets: DESCRIPTOR_POOL_SIZE,
        pool_sizes: vec![vk::DescriptorPoolSize {
            ty: vk::DescriptorType::STORAGE_BUFFER,
            descriptor_count: DESCRIPTOR_POOL_SIZE,
        }],
        ..Default::default()
    };

    let descriptor_pool = DescriptorPool::new(device, descriptor_pool_props)
        .context("creating geometry pass descriptor pool")?;

    Ok(Arc::new(descriptor_pool))
}

fn write_desc_set_primitive_ops(
    descriptor_set: &DescriptorSet,
    primitive_op_buffer: &Buffer,
) -> anyhow::Result<()> {
    let primitive_ops_buffer_info = vk::DescriptorBufferInfo {
        buffer: primitive_op_buffer.handle(),
        offset: 0,
        range: primitive_op_buffer.properties().size,
    };

    let descriptor_writes = [vk::WriteDescriptorSet::builder()
        .dst_set(descriptor_set.handle())
        .dst_binding(descriptor::BINDING_PRIMITIVE_OPS)
        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
        .buffer_info(&[primitive_ops_buffer_info])
        .build()];

    unsafe {
        descriptor_set
            .device()
            .inner()
            .update_descriptor_sets(&descriptor_writes, &[]);
    }

    Ok(())
}
