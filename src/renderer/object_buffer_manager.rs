use super::shader_interfaces::{
    primitive_op_buffer::{PrimitiveOpBufferUnit, PRIMITIVE_OP_UNIT_LEN},
    push_constants::ObjectIndexPushConstant,
    vertex_inputs::BoundingBoxVertex,
};
use crate::engine::{
    aabb::AABB_VERTEX_COUNT,
    object::object::{Object, ObjectId},
};
use anyhow::Context;
use ash::vk;
use bort::{
    Buffer, BufferProperties, CommandBuffer, DeviceOwned, GraphicsPipeline, MemoryAllocator,
    PipelineAccess,
};
use bort_vma::AllocationCreateInfo;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::{mem::size_of, sync::Arc};

// TODO biggest optimization is a staging buffer to make the vertex/storage buffers a more optimized memory type

/// Reserve for 1024 operations
const INIT_PRIMITIVE_OP_POOL_RESERVE: vk::DeviceSize =
    (1024 * PRIMITIVE_OP_UNIT_LEN * size_of::<PrimitiveOpBufferUnit>()) as vk::DeviceSize;

/// Reserve for 16 AABBs
const INIT_BOUNDING_BOX_POOL_RESERVE: vk::DeviceSize =
    (16 * AABB_VERTEX_COUNT * size_of::<BoundingBoxVertex>()) as vk::DeviceSize;

struct PerObjectBuffers {
    pub id: ObjectId,
    pub bounding_box_buffer: Arc<Buffer>,
    pub bounding_box_vertex_count: u32,
    pub primitive_ops_buffer: Arc<Buffer>,
}

/// Manages per-object resources for the geometry pass
pub struct ObjectBufferManager {
    memory_allocator: Arc<MemoryAllocator>,
    objects_buffers: Vec<PerObjectBuffers>,
}

impl ObjectBufferManager {
    pub fn new(memory_allocator: Arc<MemoryAllocator>) -> anyhow::Result<Self> {
        Ok(Self {
            memory_allocator,
            objects_buffers: Vec::new(),
        })
    }

    pub fn update_or_push(&mut self, object: &Object) -> anyhow::Result<()> {
        let id = object.id();

        let primitive_ops_buffer = upload_primitive_ops(self.memory_allocator.clone(), object)
            .context("initial upload object to buffer")?;

        if let Some(index) = self.get_index(id) {
            let bounding_box_buffer = upload_bounding_box(self.memory_allocator.clone(), object)?;

            self.objects_buffers[index].bounding_box_buffer = bounding_box_buffer;
            self.objects_buffers[index].bounding_box_vertex_count = AABB_VERTEX_COUNT as u32;
            self.objects_buffers[index].primitive_ops_buffer = primitive_ops_buffer;

            Ok(())
        } else {
            let bounding_box_buffer = upload_bounding_box(self.memory_allocator.clone(), object)?;

            let new_object = PerObjectBuffers {
                id,
                bounding_box_buffer,
                bounding_box_vertex_count: AABB_VERTEX_COUNT as u32,
                primitive_ops_buffer,
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
        for (index, per_object_buffers) in self.objects_buffers.iter().enumerate() {
            let object_index_push_constant = ObjectIndexPushConstant::new(index as u32);
            let push_constant_bytes = bytemuck::bytes_of(&object_index_push_constant);

            unsafe {
                device_ash.cmd_push_constants(
                    command_buffer_handle,
                    pipeline.pipeline_layout().handle(),
                    vk::ShaderStageFlags::VERTEX,
                    0,
                    push_constant_bytes,
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

fn upload_bounding_box(
    memory_allocator: Arc<MemoryAllocator>,
    object: &Object,
) -> anyhow::Result<Arc<Buffer>> {
    let object_id = object.id();
    trace!(
        "uploading bounding box vertices for object id = {} to gpu buffer",
        object_id
    );

    let data = object.aabb().vertices(object_id);

    let buffer_props = BufferProperties {
        size: std::mem::size_of_val(&data) as vk::DeviceSize,
        usage: vk::BufferUsageFlags::VERTEX_BUFFER,
        ..Default::default()
    };

    let alloc_info = AllocationCreateInfo {
        required_flags: vk::MemoryPropertyFlags::HOST_VISIBLE,
        preferred_flags: vk::MemoryPropertyFlags::DEVICE_LOCAL,
        ..Default::default()
    };

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
        "uploading primitive ops for object id = {} to gpu buffer",
        object.id()
    );

    let data = object.encoded_primitive_ops();

    let buffer_props = BufferProperties {
        size: std::mem::size_of_val(&data) as vk::DeviceSize,
        usage: vk::BufferUsageFlags::STORAGE_BUFFER,
        ..Default::default()
    };

    let alloc_info = AllocationCreateInfo {
        required_flags: vk::MemoryPropertyFlags::HOST_VISIBLE,
        preferred_flags: vk::MemoryPropertyFlags::DEVICE_LOCAL,
        ..Default::default()
    };

    let mut new_buffer = Buffer::new(memory_allocator, buffer_props, alloc_info)
        .context("creating geometry pass primitive op buffer")?;

    // upload data

    new_buffer
        .write_iter(data, 0)
        .context("uploading geometry pass primitive ops to buffer")?;

    Ok(Arc::new(new_buffer))
}
