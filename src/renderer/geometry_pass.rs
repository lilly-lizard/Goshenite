use super::common::{create_shader_module, CreateDescriptorSetError, CreateShaderError};
use crate::{
    config::SHADER_ENTRY_POINT,
    object::{object::Object, object_collection::ObjectCollection},
    shaders::{
        object_buffer::{ObjectDataUnit, OPERATION_UNIT_LEN},
        push_constants::CameraPushConstants,
    },
};
use anyhow::Context;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::{collections::BTreeMap, mem::size_of, sync::Arc};
use vulkano::{
    buffer::{cpu_pool::CpuBufferPoolChunk, BufferUsage, CpuBufferPool},
    command_buffer::AutoCommandBufferBuilder,
    descriptor_set::{
        allocator::StandardDescriptorSetAllocator,
        layout::{
            DescriptorSetLayout, DescriptorSetLayoutBinding, DescriptorSetLayoutCreateInfo,
            DescriptorSetLayoutCreationError, DescriptorType,
        },
        PersistentDescriptorSet, WriteDescriptorSet,
    },
    device::Device,
    memory::allocator::{AllocationCreationError, MemoryUsage, StandardMemoryAllocator},
    pipeline::{
        graphics::viewport::{Viewport, ViewportState},
        layout::{PipelineLayoutCreateInfo, PipelineLayoutCreationError},
        GraphicsPipeline, Pipeline, PipelineBindPoint, PipelineLayout,
    },
    render_pass::Subpass,
    shader::{EntryPoint, ShaderStages},
    DeviceSize,
};

const MAX_OBJECT_BUFFERS: u32 = 256;

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
    pipeline: Arc<GraphicsPipeline>,
    buffer_pool: CpuBufferPool<ObjectDataUnit>,
    object_buffers: Vec<Arc<CpuBufferPoolChunk<ObjectDataUnit>>>,
    desc_set: Arc<PersistentDescriptorSet>,
}
// Public functions
impl GeometryPass {
    pub fn new(
        device: Arc<Device>,
        memory_allocator: Arc<StandardMemoryAllocator>,
        descriptor_allocator: &StandardDescriptorSetAllocator,
        subpass: Subpass,
        object_collection: &ObjectCollection,
    ) -> anyhow::Result<Self> {
        let pipeline = create_pipeline(device.clone(), subpass)?;
        let buffer_pool = create_buffer_pool(memory_allocator)?;

        let mut object_buffers: Vec<Arc<CpuBufferPoolChunk<ObjectDataUnit>>> = Vec::new();
        for object in object_collection.objects() {
            object_buffers.push(
                upload_object(&buffer_pool, object)
                    .context("uploading initial objects to buffer")?,
            );
        }

        let desc_set = create_desc_set(descriptor_allocator, pipeline.clone(), &object_buffers)?;
        Ok(Self {
            pipeline,
            buffer_pool,
            object_buffers,
            desc_set,
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
    memory_allocator: Arc<StandardMemoryAllocator>,
) -> anyhow::Result<CpuBufferPool<ObjectDataUnit>> {
    debug!(
        "reserving {} bytes for object buffer pool",
        INIT_BUFFER_POOL_RESERVE
    );
    let buffer_pool: CpuBufferPool<ObjectDataUnit> = CpuBufferPool::new(
        memory_allocator,
        BufferUsage {
            storage_buffer: true,
            ..BufferUsage::empty()
        },
        MemoryUsage::Upload,
    );
    buffer_pool
        .reserve(INIT_BUFFER_POOL_RESERVE as u64)
        .context("reserving object buffer pool")?;
    Ok(buffer_pool)
}

fn upload_object(
    buffer_pool: &CpuBufferPool<ObjectDataUnit>,
    object: &Object,
) -> Result<Arc<CpuBufferPoolChunk<ObjectDataUnit>>, AllocationCreationError> {
    trace!("uploading object to buffer");
    buffer_pool.from_iter(object.encoded_data())
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

    let pipeline_layout = create_pipeline_layout(device.clone(), &frag_shader)
        .context("creating geometry pipeline layout")?;

    Ok(GraphicsPipeline::start()
        .render_pass(subpass)
        .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
        .vertex_shader(vert_shader, ())
        .fragment_shader(frag_shader, ())
        .with_pipeline_layout(device.clone(), pipeline_layout)
        .context("creating geometry pass pipeline")?)
}

fn create_pipeline_layout(
    device: Arc<Device>,
    frag_entry: &EntryPoint,
) -> anyhow::Result<Arc<PipelineLayout>> {
    let binding = DescriptorSetLayoutBinding {
        descriptor_count: MAX_OBJECT_BUFFERS,
        variable_descriptor_count: true,
        stages: ShaderStages {
            fragment: true,
            ..ShaderStages::empty()
        },
        ..DescriptorSetLayoutBinding::descriptor_type(DescriptorType::StorageBuffer)
    };
    let mut bindings: BTreeMap<u32, DescriptorSetLayoutBinding> = BTreeMap::new();
    bindings.insert(descriptor::BINDING_OBJECTS, binding);
    let set_layout_ci = DescriptorSetLayoutCreateInfo {
        bindings,
        push_descriptor: false,
        ..DescriptorSetLayoutCreateInfo::default()
    };
    let set_layout = DescriptorSetLayout::new(device, set_layout_ci).context("bruh")?;

    //

    let mut layout_create_infos: Vec<_> =
        DescriptorSetLayoutCreateInfo::from_requirements(frag_entry.descriptor_requirements());

    // Set 0, Binding 0
    let binding = layout_create_infos
        .get(descriptor::SET_BUFFERS)
        .ok_or(CreateDescriptorSetError::InvalidDescriptorSetIndex {
            index: descriptor::SET_BUFFERS,
            shader_path: FRAG_SHADER_PATH,
        })
        .context("bruh")?
        .bindings
        .get_mut(&descriptor::BINDING_OBJECTS)
        .context("bruh")?;
    binding.variable_descriptor_count = true;
    binding.descriptor_count = 2;

    let set_layouts = layout_create_infos
        .into_iter()
        .map(|desc| DescriptorSetLayout::new(device.clone(), desc))
        .collect::<Result<Vec<_>, DescriptorSetLayoutCreationError>>()
        .context("bruh")?;

    Ok(PipelineLayout::new(
        device.clone(),
        PipelineLayoutCreateInfo {
            set_layouts,
            push_constant_ranges: frag_entry
                .push_constant_requirements()
                .cloned()
                .into_iter()
                .collect(),
            ..Default::default()
        },
    )
    .context("bruh")?)
}

// let set_layout = geometry_pipeline todo delete
//     .layout()
//     .set_layouts()
//     .get(descriptor::SET_BUFFERS)
//     .ok_or(CreateDescriptorSetError::InvalidDescriptorSetIndex {
//         index: descriptor::SET_BUFFERS,
//         shader_path: FRAG_SHADER_PATH,
//     })
//     .context("creating object buffer desc set")?
//     .to_owned();

fn create_desc_set(
    descriptor_allocator: &StandardDescriptorSetAllocator,
    geometry_pipeline: Arc<GraphicsPipeline>,
    object_buffers: &Vec<Arc<CpuBufferPoolChunk<ObjectDataUnit>>>,
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
    PersistentDescriptorSet::new_variable(
        descriptor_allocator,
        set_layout,
        object_buffers.len() as u32,
        object_buffers
            .into_iter()
            .map(|buffer| WriteDescriptorSet::buffer(descriptor::BINDING_OBJECTS, buffer.clone()))
            .collect::<Vec<WriteDescriptorSet>>(),
    )
    .context("creating object buffer desc set")
}
