use super::{
    shader_interfaces::{
        primitive_op_buffer::{PrimitiveOpBufferUnit, PRIMITIVE_OP_UNIT_LEN},
        push_constants::ObjectIndexPushConstant,
        vertex_inputs::BoundingBoxVertex,
    },
};
use crate::engine::{
    aabb::AABB_VERTEX_COUNT,
    object::{
        object::{Object, ObjectId},
    },
};
use anyhow::Context;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::{mem::size_of, sync::Arc};
use vulkano::{
    buffer::{
        cpu_pool::CpuBufferPoolChunk, BufferUsage, CpuBufferPool,
    },
    command_buffer::AutoCommandBufferBuilder,
    memory::allocator::{AllocationCreationError, MemoryUsage, StandardMemoryAllocator},
    pipeline::{
        GraphicsPipeline, Pipeline,
    },
    DeviceSize,
};

/// Reserve for 1024 operations
const INIT_PRIMITIVE_OP_POOL_RESERVE: DeviceSize =
    (1024 * PRIMITIVE_OP_UNIT_LEN * size_of::<PrimitiveOpBufferUnit>()) as DeviceSize;

/// Reserve for 16 AABBs
const INIT_BOUNDING_BOX_POOL_RESERVE: DeviceSize =
    (16 * AABB_VERTEX_COUNT * size_of::<BoundingBoxVertex>()) as DeviceSize;

/// Manages per-object resources
pub struct ObjectBuffers {
    bounding_box_buffer_pool: CpuBufferPool<BoundingBoxVertex>,
    primitive_op_buffer_pool: CpuBufferPool<PrimitiveOpBufferUnit>,

    ids: Vec<ObjectId>,
    bounding_boxes: Vec<Arc<CpuBufferPoolChunk<BoundingBoxVertex>>>,
    bounding_box_vertex_counts: Vec<u32>,
    primitive_ops: Vec<Arc<CpuBufferPoolChunk<PrimitiveOpBufferUnit>>>,
}

impl ObjectBuffers {
    pub fn new(memory_allocator: Arc<StandardMemoryAllocator>) -> anyhow::Result<Self> {
        let bounding_box_buffer_pool = create_bounding_box_buffer_pool(memory_allocator.clone())?;
        let primitive_op_buffer_pool = create_primitive_op_buffer_pool(memory_allocator)?;

        Ok(Self {
            bounding_box_buffer_pool,
            primitive_op_buffer_pool,
            ids: Vec::new(),
            bounding_boxes: Vec::new(),
            bounding_box_vertex_counts: Vec::new(),
            primitive_ops: Vec::new(),
        })
    }

    pub fn update_or_push(&mut self, object: &Object) -> anyhow::Result<()> {
        self.debug_assert_per_object_resource_count();

        let id = object.id();

        let primitive_ops_buffer = upload_primitive_ops(&self.primitive_op_buffer_pool, object)
            .context("initial upload object to buffer")?;

        if let Some(index) = self.get_index(id) {
            let bounding_box_buffer = upload_bounding_box(&self.bounding_box_buffer_pool, object)?;

            self.bounding_boxes[index] = bounding_box_buffer;
            self.bounding_box_vertex_counts[index] = AABB_VERTEX_COUNT as u32;
            self.primitive_ops[index] = primitive_ops_buffer;

            Ok(())
        } else {
            let bounding_box_buffer = upload_bounding_box(&self.bounding_box_buffer_pool, object)?;

            self.ids.push(id);
            self.bounding_boxes.push(bounding_box_buffer);
            self.bounding_box_vertex_counts
                .push(AABB_VERTEX_COUNT as u32);
            self.primitive_ops.push(primitive_ops_buffer);

            Ok(())
        }
    }

    pub fn draw_commands<L>(
        &self,
        command_buffer: &mut AutoCommandBufferBuilder<L>,
        pipeline: Arc<GraphicsPipeline>,
    ) -> anyhow::Result<()> {
        self.debug_assert_per_object_resource_count();

        // for each object
        for index in 0..self.ids.len() {
            let object_index_push_constant = ObjectIndexPushConstant::new(index as u32);
            command_buffer
                .push_constants(pipeline.layout().clone(), 0, object_index_push_constant)
                .bind_vertex_buffers(0, self.bounding_boxes[index].clone())
                .draw(self.bounding_box_vertex_counts[index], 1, 0, 0)
                .context("recording geometry pass commands")?;
        }

        Ok(())
    }

    /// Returns the vec index if the id was found and removed.
    pub fn remove(&mut self, id: ObjectId) -> Option<usize> {
        self.debug_assert_per_object_resource_count();

        let index_res = self.get_index(id);
        if let Some(index) = index_res {
            self.ids.remove(index);
            self.bounding_boxes.remove(index);
            self.bounding_box_vertex_counts.remove(index);
            self.primitive_ops.remove(index);
        }
        index_res
    }

    pub fn get_index(&self, id: ObjectId) -> Option<usize> {
        self.ids.iter().position(|&x| x == id)
    }

    pub fn primitive_op_buffers(&self) -> &Vec<Arc<CpuBufferPoolChunk<PrimitiveOpBufferUnit>>> {
        &self.primitive_ops
    }

    pub fn bounding_box_buffers(&self) -> &Vec<Arc<CpuBufferPoolChunk<BoundingBoxVertex>>> {
        &self.bounding_boxes
    }

    #[inline]
    fn debug_assert_per_object_resource_count(&self) {
        debug_assert!(self.ids.len() == self.primitive_ops.len());
        debug_assert!(self.ids.len() == self.bounding_boxes.len());
        debug_assert!(self.ids.len() == self.bounding_box_vertex_counts.len());
    }
}

fn create_bounding_box_buffer_pool(
    memory_allocator: Arc<StandardMemoryAllocator>,
) -> anyhow::Result<CpuBufferPool<BoundingBoxVertex>> {
    debug!(
        "reserving {} bytes for bounding box buffer pool",
        INIT_BOUNDING_BOX_POOL_RESERVE
    );
    let buffer_pool: CpuBufferPool<BoundingBoxVertex> = CpuBufferPool::new(
        memory_allocator,
        BufferUsage {
            vertex_buffer: true,
            ..BufferUsage::empty()
        },
        MemoryUsage::Upload,
    );
    buffer_pool
        .reserve(INIT_BOUNDING_BOX_POOL_RESERVE)
        .context("reserving bounding box buffer pool")?;

    Ok(buffer_pool)
}

fn create_primitive_op_buffer_pool(
    memory_allocator: Arc<StandardMemoryAllocator>,
) -> anyhow::Result<CpuBufferPool<PrimitiveOpBufferUnit>> {
    debug!(
        "reserving {} bytes for primitive op buffer pool",
        INIT_PRIMITIVE_OP_POOL_RESERVE
    );
    let buffer_pool: CpuBufferPool<PrimitiveOpBufferUnit> = CpuBufferPool::new(
        memory_allocator,
        BufferUsage {
            storage_buffer: true,
            ..BufferUsage::empty()
        },
        MemoryUsage::Upload,
    );
    buffer_pool
        .reserve(INIT_PRIMITIVE_OP_POOL_RESERVE)
        .context("reserving primitive op buffer pool")?;

    Ok(buffer_pool)
}

fn upload_bounding_box(
    bounding_box_buffer_pool: &CpuBufferPool<BoundingBoxVertex>,
    object: &Object,
) -> Result<Arc<CpuBufferPoolChunk<BoundingBoxVertex>>, AllocationCreationError> {
    let object_id = object.id();
    trace!(
        "uploading bounding box vertices for object id = {} to gpu buffer",
        object_id
    );
    bounding_box_buffer_pool.from_iter(object.aabb().vertices(object_id))
}

fn upload_primitive_ops(
    primtive_op_buffer_pool: &CpuBufferPool<PrimitiveOpBufferUnit>,
    object: &Object,
) -> Result<Arc<CpuBufferPoolChunk<PrimitiveOpBufferUnit>>, AllocationCreationError> {
    trace!(
        "uploading primitive ops for object id = {} to gpu buffer",
        object.id()
    );
    primtive_op_buffer_pool.from_iter(object.encoded_primitive_ops())
}
