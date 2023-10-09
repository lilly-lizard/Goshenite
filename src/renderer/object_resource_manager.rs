use super::{
    geometry_pass::descriptor,
    shader_interfaces::{
        primitive_op_buffer::{PrimitiveOpBufferUnit, PRIMITIVE_PACKET_LEN},
        vertex_inputs::BoundingBoxVertex,
    },
};
use crate::engine::{
    aabb::AABB_VERTEX_COUNT,
    object::{
        object::{ObjectDuplicate, ObjectId},
        object_collection::ObjectCollection,
        objects_delta::{ObjectDeltaOperation, ObjectsDelta},
    },
};
use anyhow::Context;
use ash::vk;
use bort_vk::{
    allocation_info_cpu_accessible, allocation_info_from_flags, AllocAccess, Buffer,
    BufferProperties, CommandBuffer, CommandPool, CommandPoolProperties, DescriptorPool,
    DescriptorPoolProperties, DescriptorSet, DescriptorSetLayout, Device, DeviceOwned, Fence,
    GraphicsPipeline, MemoryAllocator, PipelineAccess, Queue,
};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::{mem::size_of, sync::Arc};

const DESCRIPTOR_POOL_SIZE: u32 = 256;

// TODO biggest optimization is a staging buffer to make the vertex/storage buffers a more optimized memory type

/// Reserve for 1024 primitive ops
const STAGING_BUFFER_SIZE_PRIMITIVE_OPS: vk::DeviceSize =
    (1024 * PRIMITIVE_PACKET_LEN * size_of::<PrimitiveOpBufferUnit>()) as vk::DeviceSize;

/// Reserve for 16 AABBs (one aabb per object)
const STAGING_BUFFER_SIZE_BOUNDING_MESH: vk::DeviceSize =
    (16 * AABB_VERTEX_COUNT * size_of::<BoundingBoxVertex>()) as vk::DeviceSize;

/// Time to wait when we're just checking the status of a fence.
const CHECK_FENCE_TIMEOUT_NANOSECONDS: u64 = 100;

/// Manages per-object resources for the geometry pass
pub struct ObjectResourceManager {
    device: Arc<Device>,

    memory_allocator: Arc<MemoryAllocator>,
    command_pool_transfer_queue: Arc<CommandPool>,
    command_pool_render_queue: Arc<CommandPool>,

    pending_command_resources: Vec<TransferOperationResources>,
    available_command_resources: Vec<TransferOperationResources>,

    staging_buffer_primitive_ops: Buffer,
    staging_buffer_bounding_mesh: Buffer,

    objects_buffers: Vec<PerObjectResources>,

    descriptor_pools: Vec<Arc<DescriptorPool>>,
    primitive_ops_desc_set_layout: Arc<DescriptorSetLayout>,
}

impl ObjectResourceManager {
    pub fn new(
        memory_allocator: Arc<MemoryAllocator>,
        primitive_ops_desc_set_layout: Arc<DescriptorSetLayout>,
        transfer_queue_family_index: u32,
        render_queue_family_index: u32,
    ) -> anyhow::Result<Self> {
        let device = memory_allocator.device().clone();

        let initial_descriptor_pool = create_descriptor_pool(device.clone())?;
        let command_pool_transfer_queue =
            create_command_pool(device.clone(), transfer_queue_family_index)?;
        let command_pool_render_queue =
            create_command_pool(device.clone(), render_queue_family_index)?;

        let staging_buffer_primitive_ops =
            create_staging_buffer_primitive_ops(memory_allocator.clone())?;
        let staging_buffer_bounding_mesh =
            create_staging_buffer_bounding_mesh(memory_allocator.clone())?;

        Ok(Self {
            device,

            memory_allocator,
            command_pool_transfer_queue,
            command_pool_render_queue,

            pending_command_resources: Vec::new(),
            available_command_resources: Vec::new(),

            staging_buffer_primitive_ops,
            staging_buffer_bounding_mesh,

            objects_buffers: Vec::new(),

            descriptor_pools: vec![initial_descriptor_pool],
            primitive_ops_desc_set_layout,
        })
    }

    pub fn upload_object_collection(
        &mut self,
        object_collection: &ObjectCollection,
        transfer_queue: &Queue,
        render_queue: &Queue,
    ) -> anyhow::Result<()> {
        self.reset_staging_buffer_offsets();

        let objects = object_collection.objects();

        // added objects
        for (&object_id, object) in objects {
            trace!("uploading object id = {:?} to gpu buffer", object_id);
            self.update_or_push(object_id, object.duplicate())?;
        }

        Ok(())
    }

    pub fn update_objects(
        &mut self,
        objects_delta: ObjectsDelta,
        transfer_queue: &Queue,
        render_queue: &Queue,
    ) -> anyhow::Result<()> {
        let mut transfer_operation_resources = self.get_transfer_command_resources()?;

        // begin command buffers
        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        transfer_operation_resources
            .command_buffer_transfer
            .begin(&begin_info)
            .context("beginning object transfer command buffer")?;
        transfer_operation_resources
            .command_buffer_render_sync
            .begin(&begin_info)
            .context("beginning object sync command buffer")?;

        for (object_id, object_delta) in objects_delta {
            match object_delta {
                ObjectDeltaOperation::Add(object_duplicate) => {
                    trace!("adding object id = {:?} to gpu buffer", object_id);
                    self.update_or_push(
                        object_id,
                        object_duplicate,
                        &mut transfer_operation_resources,
                    )?;
                }
                ObjectDeltaOperation::Update(object_duplicate) => {
                    trace!("updating object id = {:?} in gpu buffer", object_id);
                    self.update_or_push(
                        object_id,
                        object_duplicate,
                        &mut transfer_operation_resources,
                    )?;
                }
                ObjectDeltaOperation::Remove => {
                    if let Some(_removed_index) = self.remove(object_id) {
                        trace!("removing object buffer id = {:?}", object_id);
                    } else {
                        trace!(
                            "attempted to remove object id = {:?} from gpu buffer but not found!",
                            object_id
                        );
                    }
                }
            }
        }

        // end command buffers
        transfer_operation_resources
            .command_buffer_transfer
            .end()
            .context("ending object transfer command buffer")?;
        transfer_operation_resources
            .command_buffer_render_sync
            .end()
            .context("ending object sync command buffer")?;

        // submit upload commands
        todo!();
        let command_buffer_handles = [transfer_operation_resources
            .command_buffer_transfer
            .handle()];
        let submit_info = vk::SubmitInfo::builder().command_buffers(&command_buffer_handles);
        transfer_queue
            .submit(&[*submit_info], None)
            .context("submitting geometry buffer upload commands")?;

        self.pending_command_resources
            .push(transfer_operation_resources);

        Ok(())
    }

    pub fn update_or_push(
        &mut self,
        object_id: ObjectId,
        object: ObjectDuplicate,
        transfer_resources: &mut TransferOperationResources,
    ) -> anyhow::Result<()> {
        let primitive_ops_buffer = self
            .upload_primitive_ops(object_id, &object, transfer_resources)
            .context("initial upload object to buffer")?;

        if let Some(index) = self.get_index(object_id) {
            let bounding_mesh_buffer =
                self.upload_bounding_mesh(object_id, &object, transfer_resources)?;

            write_desc_set_primitive_ops(
                &self.objects_buffers[index].primitive_ops_descriptor_set,
                &primitive_ops_buffer,
            )?;

            self.objects_buffers[index].bounding_mesh_buffer = bounding_mesh_buffer;
            self.objects_buffers[index].bounding_mesh_vertex_count = AABB_VERTEX_COUNT as u32;
            self.objects_buffers[index].primitive_ops_buffer = primitive_ops_buffer;
        } else {
            let bounding_mesh_buffer =
                self.upload_bounding_mesh(object_id, &object, transfer_resources)?;

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

    fn get_transfer_command_resources(&mut self) -> anyhow::Result<TransferOperationResources> {
        if let Some(mut resources) = self.available_command_resources.pop() {
            // use existing resources
            resources.staging_buffer_resource_bounding_mesh.reset();
            resources.staging_buffer_resource_primitive_ops.reset();
            Ok(resources)
        } else {
            // create new resources
            let command_buffer_transfer = Arc::new(
                self.command_pool_transfer_queue
                    .allocate_command_buffer(vk::CommandBufferLevel::PRIMARY)
                    .context("allocating object upload transfer command buffer")?,
            );
            let command_buffer_render_sync = Arc::new(
                self.command_pool_render_queue
                    .allocate_command_buffer(vk::CommandBufferLevel::PRIMARY)
                    .context("allocating object upload sync command buffer")?,
            );
            let fence =
                Arc::new(Fence::new_unsignalled(self.device.clone()).context("creating fence")?);

            Ok(TransferOperationResources::new(
                command_buffer_transfer,
                command_buffer_render_sync,
                fence,
            ))
        }
    }

    fn check_and_reuse_pending_resources(&mut self) -> anyhow::Result<()> {
        let mut i: usize = 0;
        while i < self.pending_command_resources.len() {
            let wait_res = self.pending_command_resources[i]
                .fence
                .wait(CHECK_FENCE_TIMEOUT_NANOSECONDS);

            if let Err(vk_res) = wait_res {
                match vk_res {
                    // timeout means commands are still executing, can't use this one so we move on.
                    vk::Result::TIMEOUT => {
                        i = i + 1;
                        continue;
                    }
                    // error!
                    _ => anyhow::bail!(vk_res),
                };
            }

            // else: commands finished executing, can reuse these resources.
            let mut command_resources = self.pending_command_resources.remove(i);

            command_resources
                .staging_buffer_resource_bounding_mesh
                .reset();
            command_resources
                .staging_buffer_resource_primitive_ops
                .reset();

            self.available_command_resources.push(command_resources);
        }
        Ok(())
    }

    fn upload_bounding_mesh(
        &mut self,
        object_id: ObjectId,
        object: &ObjectDuplicate,
        transfer_resources: &mut TransferOperationResources,
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
                transfer_resources,
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
        transfer_resources: &mut TransferOperationResources,
    ) -> anyhow::Result<()>
    where
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        self.staging_buffer_bounding_mesh
            .write_iter(data, self.staging_buffer_offset_bounding_mesh as usize)
            .context("uploading geometry pass bounding box vertices to staging buffer")?;

        let after_transfer_barrier = vk::BufferMemoryBarrier::builder()
            .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .dst_access_mask(vk::AccessFlags::VERTEX_ATTRIBUTE_READ)
            .buffer(new_buffer.handle())
            .size(data_size)
            .offset(0)
            .build();

        let copy_region = vk::BufferCopy {
            src_offset: self.staging_buffer_offset_bounding_mesh,
            dst_offset: 0,
            size: data_size,
        };

        unsafe {
            self.device.inner().cmd_copy_buffer(
                command_buffer.handle(),
                self.staging_buffer_bounding_mesh.handle(),
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

        self.staging_buffer_offset_bounding_mesh += data_size;

        Ok(())
    }

    fn upload_primitive_ops(
        &mut self,
        object_id: ObjectId,
        object: &ObjectDuplicate,
        transfer_resources: &mut TransferOperationResources,
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

        self.staging_buffer_primitive_ops
            .write_iter(data, self.staging_buffer_offset_primitive_ops as usize)
            .context("uploading geometry pass primitive ops to staging buffer")?;

        let after_transfer_barrier = vk::BufferMemoryBarrier::builder()
            .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .buffer(new_buffer.handle())
            .size(data_size)
            .offset(0)
            .build();

        let before_render_barrier = vk::BufferMemoryBarrier::builder()
            .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .buffer(new_buffer.handle())
            .size(data_size)
            .offset(0)
            .build();

        let copy_region = vk::BufferCopy {
            src_offset: self.staging_buffer_offset_primitive_ops,
            dst_offset: 0,
            size: data_size,
        };

        unsafe {
            self.device.inner().cmd_copy_buffer(
                command_buffer.handle(),
                self.staging_buffer_primitive_ops.handle(),
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
        self.staging_buffer_offset_primitive_ops += data_size;

        Ok(Arc::new(new_buffer))
    }

    fn determine_staging_buffer_offset_primitive_ops(
        &self,
        upload_data_size: vk::DeviceSize,
    ) -> Option<vk::DeviceSize> {
        let staging_buffer_size = self.staging_buffer_primitive_ops.properties().size;

        let mut regions_in_use = self
            .pending_command_resources
            .iter()
            .filter_map(|r| match r.staging_buffer_resource_primitive_ops {
                StagingBufferResources::PreAllocatedRegion(region) => Some(region),
                _ => None,
            })
            .collect::<Vec<BufferRegion>>();

        self.determine_staging_buffer_offset(upload_data_size, regions_in_use, staging_buffer_size)
    }

    fn determine_staging_buffer_offset_bounding_mesh(
        &self,
        upload_data_size: vk::DeviceSize,
    ) -> Option<vk::DeviceSize> {
        let staging_buffer_size = self.staging_buffer_bounding_mesh.properties().size;

        let mut regions_in_use = self
            .pending_command_resources
            .iter()
            .filter_map(|r| match r.staging_buffer_resource_bounding_mesh {
                StagingBufferResources::PreAllocatedRegion(region) => Some(region),
                _ => None,
            })
            .collect::<Vec<BufferRegion>>();

        self.determine_staging_buffer_offset(upload_data_size, regions_in_use, staging_buffer_size)
    }

    fn determine_staging_buffer_offset(
        &self,
        upload_data_size: vk::DeviceSize,
        mut regions_in_use: Vec<BufferRegion>,
        staging_buffer_size: vk::DeviceSize,
    ) -> Option<vk::DeviceSize> {
        // sort by start position (`Ord` is implemented for `BufferRegion` by comparing the start position)
        regions_in_use.sort();

        if regions_in_use.len() == 0 {
            // no regions in use so just start from beginning of the staging buffer
            return Some(0);
        }

        if regions_in_use[0].start > upload_data_size {
            // enough bytes before first staging buffer region to use starting space
            return Some(0);
        }

        // check spaces inbetween regions
        for i in 0..(regions_in_use.len() - 1) {
            let current_region = regions_in_use[i];
            let next_region = regions_in_use[i + 1];

            let current_region_end = current_region.start + current_region.size;
            let space_between_regions = next_region.start - current_region_end;

            if upload_data_size <= space_between_regions {
                return Some(current_region_end);
            }
        }

        // check space after last region in use
        let last_region = regions_in_use[regions_in_use.len() - 1];
        let last_region_end = last_region.start + last_region.size;
        let space_after_last_region = staging_buffer_size - last_region_end;
        if upload_data_size <= space_after_last_region {
            return Some(last_region_end);
        }

        // no region of contiguous data available in staging buffer
        None
    }
}

fn create_command_pool(
    device: Arc<Device>,
    queue_family_index: u32,
) -> anyhow::Result<Arc<CommandPool>> {
    let command_pool_props = CommandPoolProperties {
        flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
        queue_family_index,
    };

    let command_pool = CommandPool::new(device, command_pool_props)
        .context("creating gui renderer command pool")?;

    Ok(Arc::new(command_pool))
}

fn create_staging_buffer_primitive_ops(
    memory_allocator: Arc<MemoryAllocator>,
) -> anyhow::Result<Buffer> {
    let buffer_props = BufferProperties::new_default(
        STAGING_BUFFER_SIZE_PRIMITIVE_OPS,
        vk::BufferUsageFlags::TRANSFER_SRC,
    );
    let alloc_info = allocation_info_cpu_accessible();

    Buffer::new(memory_allocator, buffer_props, alloc_info)
        .context("creating primitive op staging buffer")
}

fn create_staging_buffer_bounding_mesh(
    memory_allocator: Arc<MemoryAllocator>,
) -> anyhow::Result<Buffer> {
    let buffer_props = BufferProperties::new_default(
        STAGING_BUFFER_SIZE_BOUNDING_MESH,
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

// ~~ Helper Structs ~~

struct PerObjectResources {
    pub object_id: ObjectId,
    pub bounding_mesh_buffer: Arc<Buffer>,
    pub bounding_mesh_vertex_count: u32,
    pub primitive_ops_buffer: Arc<Buffer>,
    pub primitive_ops_descriptor_set: Arc<DescriptorSet>,
}

struct TransferOperationResources {
    pub command_buffer_transfer: Arc<CommandBuffer>,
    pub command_buffer_render_sync: Arc<CommandBuffer>,
    pub fence: Arc<Fence>,
    pub staging_buffer_resource_primitive_ops: StagingBufferResources,
    pub staging_buffer_resource_bounding_mesh: StagingBufferResources,
}

impl TransferOperationResources {
    pub fn new(
        command_buffer_transfer: Arc<CommandBuffer>,
        command_buffer_render_sync: Arc<CommandBuffer>,
        fence: Arc<Fence>,
    ) -> Self {
        Self {
            command_buffer_transfer,
            command_buffer_render_sync,
            fence,
            staging_buffer_resource_bounding_mesh: Default::default(),
            staging_buffer_resource_primitive_ops: Default::default(),
        }
    }
}

enum StagingBufferResources {
    /// A region of the large pre-allocated staging buffer is used
    PreAllocatedRegion(BufferRegion),
    /// There was not enough space in the pre-allocated buffer so a new one was created just for
    /// this upload
    DedicatedStagingBuffer(Arc<Buffer>),
}

impl Default for StagingBufferResources {
    fn default() -> Self {
        Self::PreAllocatedRegion(Default::default())
    }
}

impl StagingBufferResources {
    pub fn reset(&mut self) {
        *self = Default::default()
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Ord)]
/// Describes a sub-region of a larger allocation contiguous data
struct BufferRegion {
    /// Position of first byte
    pub start: vk::DeviceSize,
    /// Number of bytes used
    pub size: vk::DeviceSize,
}

impl BufferRegion {
    pub fn reset(&mut self) {
        self.start = 0;
        self.size = 0;
    }
}

// just order by start position as we should only compare regions for the same buffer in which case
// there shouldn't be any overlap
impl PartialOrd for BufferRegion {
    fn ge(&self, other: &Self) -> bool {
        self.start >= other.start
    }

    fn gt(&self, other: &Self) -> bool {
        self.start > other.start
    }

    fn le(&self, other: &Self) -> bool {
        self.start <= other.start
    }

    fn lt(&self, other: &Self) -> bool {
        self.start < other.start
    }

    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.start.partial_cmp(&other.start)
    }
}
