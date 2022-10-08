use super::common::{create_shader_module, CreateDescriptorSetError, CreateShaderError};
use crate::{
    camera::Camera,
    config,
    primitives::primitive_collection::PrimitiveCollection,
    shaders::shader_interfaces::{
        CameraPushConstants, PrimitiveData, PrimitiveDataUnit, SHADER_ENTRY_POINT,
    },
};
use anyhow::Context;
#[allow(unused_imports)]
use log::{debug, error, info, warn};
use std::sync::Arc;
use vulkano::{
    buffer::{cpu_pool::CpuBufferPoolChunk, BufferUsage, CpuBufferPool},
    command_buffer::AutoCommandBufferBuilder,
    descriptor_set::{
        allocator::StandardDescriptorSetAllocator, PersistentDescriptorSet, WriteDescriptorSet,
    },
    device::Device,
    memory::pool::StandardMemoryPool,
    pipeline::{
        graphics::viewport::{Viewport, ViewportState},
        GraphicsPipeline, Pipeline, PipelineBindPoint,
    },
    render_pass::Subpass,
    DeviceSize,
};

const VERT_SHADER_PATH: &str = "assets/shader_binaries/full_screen.vert.cxx.spv";
const FRAG_SHADER_PATH: &str = "assets/shader_binaries/scene_geometry.frag.spv";

/// Describes descriptor set indices
mod descriptor {
    pub const SET_PRIMITVES: usize = 0;
    pub const BINDING_PRIMITVES: u32 = 0;
}

/// The initial primitive buffer pool allocation
const RESERVED_PRIMITIVE_BUFFER_POOL: DeviceSize = 8 * 4 * 1024;

/// Defines functionality for rendering the scene geometry to write to g-buffers
pub struct GeometryPass {
    primitive_buffer_pool: CpuBufferPool<PrimitiveDataUnit>,
    pipeline: Arc<GraphicsPipeline>,
    desc_set_primitives: Arc<PersistentDescriptorSet>,
}
// Public functions
impl GeometryPass {
    pub fn new(
        device: Arc<Device>,
        descriptor_allocator: &StandardDescriptorSetAllocator,
        primitive_collection: &PrimitiveCollection,
        subpass: Subpass,
    ) -> anyhow::Result<Self> {
        // init primitive buffer pool
        let primitive_buffer_pool = CpuBufferPool::new(
            device.clone(),
            BufferUsage {
                storage_buffer: true,
                ..BufferUsage::empty()
            },
        );
        primitive_buffer_pool
            .reserve(RESERVED_PRIMITIVE_BUFFER_POOL)
            .context("reserving primitive buffer pool")?;
        debug!(
            "reserving {} bytes for primitives buffer pool",
            RESERVED_PRIMITIVE_BUFFER_POOL
        );

        // init compute pipeline and descriptor sets
        let pipeline = create_pipeline(device.clone(), subpass)?;
        let primitive_buffer =
            create_primitives_buffer(primitive_collection, &primitive_buffer_pool)?;
        let desc_set_primitives =
            create_desc_set_primitives(descriptor_allocator, pipeline.clone(), primitive_buffer)?;

        Ok(Self {
            pipeline,
            desc_set_primitives,
            primitive_buffer_pool,
        })
    }

    /// Update the primitives storage buffer.
    //todo shoul be optimized to not create a new buffer each time...
    pub fn update_primitives(
        &mut self,
        descriptor_allocator: &StandardDescriptorSetAllocator,
        primitive_collection: &PrimitiveCollection,
    ) -> anyhow::Result<()> {
        let primitive_buffer =
            create_primitives_buffer(primitive_collection, &self.primitive_buffer_pool)?;
        self.desc_set_primitives = create_desc_set_primitives(
            descriptor_allocator,
            self.pipeline.clone(),
            primitive_buffer,
        )?;
        Ok(())
    }

    /// Records rendering commands to a command buffer
    pub fn record_commands<L>(
        &self,
        command_buffer: &mut AutoCommandBufferBuilder<L>,
        camera: &Camera,
        viewport: Viewport,
    ) -> anyhow::Result<()> {
        let camera_push_constant = CameraPushConstants::new(
            glam::Mat4::inverse(&(camera.proj_matrix() * camera.view_matrix())),
            camera.position(),
        );
        command_buffer
            .set_viewport(0, [viewport])
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                self.desc_set_primitives.clone(),
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

fn create_desc_set_primitives(
    descriptor_allocator: &StandardDescriptorSetAllocator,
    geometry_pipeline: Arc<GraphicsPipeline>,
    primitive_buffer: Arc<CpuBufferPoolChunk<PrimitiveDataUnit, Arc<StandardMemoryPool>>>,
) -> anyhow::Result<Arc<PersistentDescriptorSet>> {
    PersistentDescriptorSet::new(
        descriptor_allocator,
        geometry_pipeline
            .layout()
            .set_layouts()
            .get(descriptor::SET_PRIMITVES)
            .ok_or(CreateDescriptorSetError::InvalidDescriptorSetIndex {
                index: descriptor::SET_PRIMITVES,
                shader_path: FRAG_SHADER_PATH,
            })
            .context("creating primitives desc set")?
            .to_owned(),
        [WriteDescriptorSet::buffer(
            descriptor::BINDING_PRIMITVES,
            primitive_buffer,
        )],
    )
    .context("creating primitives desc set")
}

fn create_primitives_buffer(
    primitive_collection: &PrimitiveCollection,
    buffer_pool: &CpuBufferPool<PrimitiveDataUnit>,
) -> anyhow::Result<Arc<CpuBufferPoolChunk<PrimitiveDataUnit, Arc<StandardMemoryPool>>>> {
    // todo should be able to update buffer wihtout recreating?
    let combined_data = PrimitiveData::combined_data(primitive_collection)?;
    if config::PER_FRAME_DEBUG_LOGS {
        debug!(
            "creating new primitives buffer slice for {} primitives",
            combined_data.len()
        );
    }
    buffer_pool
        .from_iter(combined_data)
        .context("creating primitives buffer")
}
