use super::common::{create_shader_module, CreateDescriptorSetError, CreateShaderError};
use crate::{
    config::SHADER_ENTRY_POINT,
    object::object::Object,
    shaders::{
        object_buffer::{ObjectDataUnit, OPERATION_UNIT_LEN},
        push_constants::CameraPushConstants,
    },
};
use anyhow::Context;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::{
    mem::{align_of, size_of},
    ptr,
    sync::Arc,
};
use vulkano::{
    buffer::{cpu_access::WriteLockError, BufferUsage, CpuAccessibleBuffer},
    command_buffer::AutoCommandBufferBuilder,
    descriptor_set::{
        allocator::StandardDescriptorSetAllocator, PersistentDescriptorSet, WriteDescriptorSet,
    },
    device::Device,
    memory::allocator::MemoryAllocator,
    pipeline::{
        graphics::viewport::{Viewport, ViewportState},
        GraphicsPipeline, Pipeline, PipelineBindPoint,
    },
    render_pass::Subpass,
    DeviceSize,
};

const VERT_SHADER_PATH: &str = "assets/shader_binaries/bounding_box.vert.spv";
const FRAG_SHADER_PATH: &str = "assets/shader_binaries/scene_geometry.frag.spv";

// descriptor set and binding indices
mod descriptor {
    pub const SET_BUFFERS: usize = 0;
    pub const BINDING_OBJECTS: u32 = 0;
}

/// start out with 1024 operations
const INIT_BUFFER_POOL_RESERVE: DeviceSize =
    (1024 * OPERATION_UNIT_LEN * size_of::<ObjectDataUnit>()) as DeviceSize;

/// Render the scene geometry and write to g-buffers
pub struct GeometryPass {
    buffer_pool: Arc<CpuAccessibleBuffer<[ObjectDataUnit]>>,
    pipeline: Arc<GraphicsPipeline>,
    desc_set: Arc<PersistentDescriptorSet>,
}
// Public functions
impl GeometryPass {
    pub fn new(
        device: Arc<Device>,
        memory_allocator: &impl MemoryAllocator,
        descriptor_allocator: &StandardDescriptorSetAllocator,
        subpass: Subpass,
        objects: &Vec<Object>,
    ) -> anyhow::Result<Self> {
        let buffer_pool = create_buffer_pool(memory_allocator)?;
        debug!("uploading initial object to object buffer pool",);
        upload_object(&buffer_pool, object).context("uploading initial object to buffer")?;

        let pipeline = create_pipeline(device.clone(), subpass)?;
        let desc_set =
            create_desc_set(descriptor_allocator, pipeline.clone(), buffer_pool.clone())?;

        Ok(Self {
            pipeline,
            desc_set,
            buffer_pool,
        })
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

fn create_buffer_pool(
    memory_allocator: &impl MemoryAllocator,
) -> anyhow::Result<Arc<CpuAccessibleBuffer<[ObjectDataUnit]>>> {
    debug!(
        "reserving {} bytes for object buffer pool",
        INIT_BUFFER_POOL_RESERVE
    );
    unsafe {
        CpuAccessibleBuffer::raw(
            memory_allocator,
            INIT_BUFFER_POOL_RESERVE,
            align_of::<ObjectDataUnit>() as DeviceSize,
            BufferUsage {
                storage_buffer: true,
                ..BufferUsage::empty()
            },
            false,
            [],
        )
        .context("reserving object buffer pool")
    }
}

fn upload_object(
    buffer_pool: &CpuAccessibleBuffer<[ObjectDataUnit]>,
    object: &Object,
) -> Result<DeviceSize, WriteLockError> {
    unsafe {
        let mut mapping = buffer_pool.write()?;
        // todo optimize? see CpuAccessibleBuffer::from_data...
        for (i, o) in object.encoded_data().into_iter().zip(mapping.iter_mut()) {
            ptr::write(o, i);
        }
    }
    Ok(0)
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
    buffer_pool: Arc<CpuAccessibleBuffer<[ObjectDataUnit]>>,
) -> anyhow::Result<Arc<PersistentDescriptorSet>> {
    let set_layout = geometry_pipeline
        .layout()
        .set_layouts()
        .get(descriptor::SET_BUFFERS)
        .ok_or(CreateDescriptorSetError::InvalidDescriptorSetIndex {
            index: descriptor::SET_BUFFERS,
            shader_path: FRAG_SHADER_PATH,
        })
        .context("creating object buffer desc set")?
        .to_owned();
    PersistentDescriptorSet::new(
        descriptor_allocator,
        set_layout,
        [WriteDescriptorSet::buffer(
            descriptor::BINDING_OBJECTS,
            buffer_pool,
        )],
    )
    .context("creating object buffer desc set")
}
