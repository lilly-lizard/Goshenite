use super::common::{create_shader_module, CreateDescriptorSetError, CreateShaderError};
use crate::{
    config::SHADER_ENTRY_POINT,
    object::object::Object,
    shaders::{object_buffer::ObjectDataUnit, push_constants::CameraPushConstants},
};
use anyhow::Context;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::sync::Arc;
use vulkano::{
    buffer::{cpu_pool::CpuBufferPoolChunk, BufferUsage, CpuBufferPool},
    command_buffer::AutoCommandBufferBuilder,
    descriptor_set::{
        allocator::StandardDescriptorSetAllocator, PersistentDescriptorSet, WriteDescriptorSet,
    },
    device::Device,
    memory::allocator::StandardMemoryAllocator,
    pipeline::{
        graphics::viewport::{Viewport, ViewportState},
        GraphicsPipeline, Pipeline, PipelineBindPoint,
    },
    render_pass::Subpass,
    DeviceSize,
};

const VERT_SHADER_PATH: &str = "assets/shader_binaries/full_screen.vert.spv";
const FRAG_SHADER_PATH: &str = "assets/shader_binaries/scene_geometry.frag.spv";

// descriptor set and binding indices
mod descriptor {
    pub const SET_BUFFERS: usize = 0;
    pub const BINDING_PRIMITIVES: u32 = 0;
    pub const BINDING_OPERATIONS: u32 = 1;
}

const RESERVED_BUFFER_POOL: DeviceSize = 8 * 4 * 1024;

/// Render the scene geometry and write to g-buffers
pub struct GeometryPass {
    buffer_pool: CpuBufferPool<u32>,
    pipeline: Arc<GraphicsPipeline>,
    desc_set: Arc<PersistentDescriptorSet>,
}
// Public functions
impl GeometryPass {
    pub fn new(
        device: Arc<Device>,
        descriptor_allocator: &StandardDescriptorSetAllocator,
        object: &Object, // todo just 1 object for now
        subpass: Subpass,
    ) -> anyhow::Result<Self> {
        let buffer_pool = CpuBufferPool::new(
            device.clone(),
            BufferUsage {
                storage_buffer: true,
                ..BufferUsage::empty()
            },
        );
        buffer_pool
            .reserve(RESERVED_BUFFER_POOL)
            .context("reserving primitive buffer pool")?;
        debug!(
            "reserving {} bytes for primitives buffer pool",
            RESERVED_BUFFER_POOL
        );

        let pipeline = create_pipeline(device.clone(), subpass)?;

        let primitives_buffer = create_primitives_buffer(primitive_collection, &buffer_pool)?;
        let operations_buffer = create_operations_buffer(operation_collection, &buffer_pool)?;
        let desc_set = create_desc_set(
            descriptor_allocator,
            pipeline.clone(),
            primitives_buffer,
            operations_buffer,
        )?;

        Ok(Self {
            pipeline,
            desc_set,
            buffer_pool,
        })
    }

    /// Update the primitives storage buffer.
    //todo should be optimized to not create a new buffer each time...
    pub fn update_buffers(
        &mut self,
        descriptor_allocator: &StandardDescriptorSetAllocator,
        primitive_collection: &PrimitiveCollection,
        operation_collection: &OperationCollection,
    ) -> anyhow::Result<()> {
        let primitives_buffer = create_primitives_buffer(primitive_collection, &self.buffer_pool)?;
        let operations_buffer = create_operations_buffer(operation_collection, &self.buffer_pool)?;
        let desc_set = create_desc_set(
            descriptor_allocator,
            self.pipeline.clone(),
            primitives_buffer,
            operations_buffer,
        )?;
        self.desc_set = desc_set;
        Ok(())
    }

    pub fn record_commands<L>(
        &self,
        command_buffer: &mut AutoCommandBufferBuilder<L>,
        camera_push_constant: CameraPushConstants,
        viewport: Viewport,
    ) -> anyhow::Result<()> {
        command_buffer
            .set_viewport(0, [viewport])
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                self.desc_set.clone(),
            )
            .push_constants(self.pipeline.layout().clone(), 0, camera_push_constant)
            .draw(3, 1, 0, 0)
            .context("recording geometry pass commands")?;
        Ok(())
    }
}

fn create_pipeline(device: Arc<Device>, subpass: Subpass) -> anyhow::Result<Arc<GraphicsPipeline>> {
    let vert_module = create_shader_module(device.clone(), VERT_SHADER_PATH)?;
    let vert_shader =
        vert_module
            .entry_point(SHADER_ENTRY_POINT)
            .ok_or(CreateShaderError::MissingEntryPoint(
                VERT_SHADER_PATH.to_owned(),
            ))?;
    let frag_module = create_shader_module(device.clone(), FRAG_SHADER_PATH)?;
    let frag_shader =
        frag_module
            .entry_point(SHADER_ENTRY_POINT)
            .ok_or(CreateShaderError::MissingEntryPoint(
                FRAG_SHADER_PATH.to_owned(),
            ))?;
    Ok(GraphicsPipeline::start()
        .render_pass(subpass)
        .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
        .vertex_shader(vert_shader, ())
        .fragment_shader(frag_shader, ())
        .build(device.clone())
        .context("creating geometry pass pipeline")?)
}

fn create_desc_set(
    descriptor_allocator: &StandardDescriptorSetAllocator,
    geometry_pipeline: Arc<GraphicsPipeline>,
    primitives_buffer: Arc<CpuBufferPoolChunk<PrimitiveDataUnit, Arc<StandardMemoryPool>>>,
    operations_buffer: Arc<CpuBufferPoolChunk<OperationDataUnit, Arc<StandardMemoryPool>>>,
) -> anyhow::Result<Arc<PersistentDescriptorSet>> {
    PersistentDescriptorSet::new(
        descriptor_allocator,
        geometry_pipeline
            .layout()
            .set_layouts()
            .get(descriptor::SET_BUFFERS)
            .ok_or(CreateDescriptorSetError::InvalidDescriptorSetIndex {
                index: descriptor::SET_BUFFERS,
                shader_path: FRAG_SHADER_PATH,
            })
            .context("creating primitives desc set")?
            .to_owned(),
        [
            WriteDescriptorSet::buffer(descriptor::BINDING_PRIMITIVES, primitives_buffer),
            WriteDescriptorSet::buffer(descriptor::BINDING_OPERATIONS, operations_buffer),
        ],
    )
    .context("creating primitives desc set")
}

fn create_primitives_buffer(
    primitive_collection: &PrimitiveCollection,
    buffer_pool: &CpuBufferPool<PrimitiveDataUnit>,
) -> anyhow::Result<Arc<CpuBufferPoolChunk<PrimitiveDataUnit, Arc<StandardMemoryPool>>>> {
    // todo should be able to update buffer wihtout recreating?
    let buffer_data: Vec<PrimitiveDataUnit> = primitive_collection
        .buffer_data()
        .into_iter()
        .flatten()
        .copied()
        .collect();
    trace!(
        "creating new primitives buffer slice for {} primitives",
        buffer_data.len()
    );
    buffer_pool
        .from_iter(buffer_data)
        .context("creating primitives buffer")
}

fn create_operations_buffer(
    operations_collection: &OperationCollection,
    buffer_pool: &CpuBufferPool<OperationDataUnit>,
) -> anyhow::Result<Arc<CpuBufferPoolChunk<OperationDataUnit, Arc<StandardMemoryPool>>>> {
    // todo should be able to update buffer wihtout recreating?
    let buffer_data = operation_buffer::to_raw_buffer(operations_collection)?;
    trace!(
        "creating new operations buffer slice for {} operations",
        buffer_data.len()
    );
    buffer_pool
        .from_iter(buffer_data)
        .context("creating operations buffer")
}
