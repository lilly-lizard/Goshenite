use super::{
    geometry_pass::descriptor,
    shader_interfaces::{
        primitive_op_buffer::{PrimitiveOpBufferUnit, PRIMITIVE_PACKET_LEN},
        vertex_inputs::BoundingBoxVertex,
    },
};
use crate::engine::{
    aabb::AABB_VERTEX_COUNT,
    object::object::{Object, ObjectId},
};
use anyhow::Context;
use ash::vk;
use bort::{
    allocation_info_from_flags, AllocAccess, Buffer, BufferProperties, CommandBuffer,
    DescriptorPool, DescriptorPoolProperties, DescriptorSet, DescriptorSetLayout, Device,
    DeviceOwned, GraphicsPipeline, MemoryAllocator, PipelineAccess,
};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::{mem::size_of, sync::Arc};

const DESCRIPTOR_POOL_SIZE: u32 = 256;

// TODO biggest optimization is a staging buffer to make the vertex/storage buffers a more optimized memory type

/// Reserve for 1024 primitive ops
const INIT_PRIMITIVE_OP_POOL_RESERVE: vk::DeviceSize =
    (1024 * PRIMITIVE_PACKET_LEN * size_of::<PrimitiveOpBufferUnit>()) as vk::DeviceSize;

/// Reserve for 16 AABBs (one aabb per object)
const INIT_BOUNDING_BOX_POOL_RESERVE: vk::DeviceSize =
    (16 * AABB_VERTEX_COUNT * size_of::<BoundingBoxVertex>()) as vk::DeviceSize;

struct PerObjectResources {
    pub id: ObjectId,
    pub bounding_box_buffer: Arc<Buffer>,
    pub bounding_box_vertex_count: u32,
    pub primitive_ops_buffer: Arc<Buffer>,
    pub primitive_ops_descriptor_set: Arc<DescriptorSet>,
}

/// Manages per-object resources for the geometry pass
pub struct ObjectResourceManager {
    memory_allocator: Arc<MemoryAllocator>,
    objects_buffers: Vec<PerObjectResources>,
    descriptor_pools: Vec<Arc<DescriptorPool>>,
    primitive_ops_desc_set_layout: Arc<DescriptorSetLayout>,
}

impl ObjectResourceManager {
    pub fn new(
        memory_allocator: Arc<MemoryAllocator>,
        primitive_ops_desc_set_layout: Arc<DescriptorSetLayout>,
    ) -> anyhow::Result<Self> {
        let initial_descriptor_pool = create_descriptor_pool(memory_allocator.device().clone())?;

        Ok(Self {
            memory_allocator,
            objects_buffers: Vec::new(),
            descriptor_pools: vec![initial_descriptor_pool],
            primitive_ops_desc_set_layout,
        })
    }

    pub fn update_or_push(&mut self, object: &Object) -> anyhow::Result<()> {
        let id = object.id();

        let primitive_ops_buffer = upload_primitive_ops(self.memory_allocator.clone(), object)
            .context("initial upload object to buffer")?;

        if let Some(index) = self.get_index(id) {
            let bounding_box_buffer = upload_bounding_box(self.memory_allocator.clone(), object)?;

            write_desc_set_primitive_ops(
                &self.objects_buffers[index].primitive_ops_descriptor_set,
                &primitive_ops_buffer,
            )?;

            self.objects_buffers[index].bounding_box_buffer = bounding_box_buffer;
            self.objects_buffers[index].bounding_box_vertex_count = AABB_VERTEX_COUNT as u32;
            self.objects_buffers[index].primitive_ops_buffer = primitive_ops_buffer;

            Ok(())
        } else {
            let bounding_box_buffer = upload_bounding_box(self.memory_allocator.clone(), object)?;

            let primitive_ops_descriptor_set = self.allocate_primitive_ops_descriptor_set()?;
            write_desc_set_primitive_ops(&primitive_ops_descriptor_set, &primitive_ops_buffer)?;

            let new_object = PerObjectResources {
                id,
                bounding_box_buffer,
                bounding_box_vertex_count: AABB_VERTEX_COUNT as u32,
                primitive_ops_buffer,
                primitive_ops_descriptor_set,
            };
            self.objects_buffers.push(new_object);

            Ok(())
        }
    }

    pub fn draw_commands(
        &self,
        command_buffer: &CommandBuffer,
        pipeline: &GraphicsPipeline,
    ) -> anyhow::Result<()> {
        let device_ash = command_buffer.device().inner();
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
                    &[per_object_buffers.bounding_box_buffer.handle()],
                    &[0],
                );
                device_ash.cmd_draw(
                    command_buffer_handle,
                    per_object_buffers.bounding_box_vertex_count,
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

    pub fn get_index(&self, id: ObjectId) -> Option<usize> {
        self.objects_buffers.iter().position(|o| o.id == id)
    }

    pub fn primitive_op_buffers(&self) -> Vec<Arc<Buffer>> {
        self.objects_buffers
            .iter()
            .map(|o| o.primitive_ops_buffer.clone())
            .collect::<Vec<_>>()
    }

    pub fn bounding_box_buffers(&self) -> Vec<Arc<Buffer>> {
        self.objects_buffers
            .iter()
            .map(|o| o.bounding_box_buffer.clone())
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
                    todo!();
                } else {
                    return Err(alloc_err).context("allocating primitive ops desc set");
                }
            }
            Ok(desc_set) => desc_set,
        };

        Ok(Arc::new(desc_set))
    }
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

fn upload_bounding_box(
    memory_allocator: Arc<MemoryAllocator>,
    object: &Object,
) -> anyhow::Result<Arc<Buffer>> {
    let object_id = object.id();
    trace!(
        "uploading bounding box vertices for object id = {:?} to gpu buffer",
        object_id
    );

    let data = object.aabb().vertices(object_id);

    let buffer_props = BufferProperties::new_default(
        std::mem::size_of_val(&data) as vk::DeviceSize,
        vk::BufferUsageFlags::VERTEX_BUFFER,
    );

    let alloc_info = allocation_info_from_flags(
        vk::MemoryPropertyFlags::HOST_VISIBLE,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    );

    let mut new_buffer = Buffer::new(memory_allocator, buffer_props, alloc_info)
        .context("creating geometry pass bounding box buffer")?;

    // upload data

    new_buffer
        .write_iter(data, 0)
        .context("uploading geometry pass bounding box vertices to buffer")?;

    Ok(Arc::new(new_buffer))
}

fn upload_primitive_ops(
    memory_allocator: Arc<MemoryAllocator>,
    object: &Object,
) -> anyhow::Result<Arc<Buffer>> {
    trace!(
        "uploading primitive ops for object id = {:?} to gpu buffer",
        object.id()
    );

    let data = object.encoded_primitive_ops();

    let buffer_props = BufferProperties::new_default(
        std::mem::size_of_val(data.as_slice()) as vk::DeviceSize,
        vk::BufferUsageFlags::STORAGE_BUFFER,
    );

    let alloc_info = allocation_info_from_flags(
        vk::MemoryPropertyFlags::HOST_VISIBLE,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    );

    let mut new_buffer = Buffer::new(memory_allocator, buffer_props, alloc_info)
        .context("creating geometry pass primitive op buffer")?;

    // upload data

    new_buffer
        .write_iter(data, 0)
        .context("uploading geometry pass primitive ops to buffer")?;

    Ok(Arc::new(new_buffer))
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
