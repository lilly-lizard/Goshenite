use super::{
    common::{create_shader_module, CreateDescriptorSetError, CreateShaderError},
    renderer_config::SHADER_ENTRY_POINT,
    shader_interfaces::{
        object_buffer::{ObjectDataUnit, OPERATION_UNIT_LEN},
        push_constants::CameraPushConstants,
    },
};
use crate::engine::object::{
    object::{Object, ObjectId},
    object_collection::ObjectCollection,
    objects_delta::ObjectsDelta,
};
use anyhow::Context;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::{borrow::Borrow, mem::size_of, sync::Arc};
use vulkano::{
    buffer::{cpu_pool::CpuBufferPoolChunk, BufferAccess, BufferUsage, CpuBufferPool},
    command_buffer::AutoCommandBufferBuilder,
    descriptor_set::{
        allocator::StandardDescriptorSetAllocator,
        layout::{
            DescriptorSetLayout, DescriptorSetLayoutCreateInfo, DescriptorSetLayoutCreationError,
        },
        PersistentDescriptorSet, WriteDescriptorSet,
    },
    device::Device,
    memory::allocator::{AllocationCreationError, MemoryUsage, StandardMemoryAllocator},
    pipeline::{
        graphics::viewport::{Viewport, ViewportState},
        layout::PipelineLayoutCreateInfo,
        GraphicsPipeline, Pipeline, PipelineBindPoint, PipelineLayout,
    },
    render_pass::Subpass,
    shader::EntryPoint,
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
    descriptor_allocator: Arc<StandardDescriptorSetAllocator>,
    pipeline: Arc<GraphicsPipeline>,

    object_buffer_pool: CpuBufferPool<ObjectDataUnit>,
    object_buffers: ObjectBuffers,
    desc_set: Arc<PersistentDescriptorSet>,
}
// Public functions
impl GeometryPass {
    pub fn new(
        device: Arc<Device>,
        memory_allocator: Arc<StandardMemoryAllocator>,
        descriptor_allocator: Arc<StandardDescriptorSetAllocator>,
        subpass: Subpass,
        object_collection: &ObjectCollection,
    ) -> anyhow::Result<Self> {
        let pipeline = create_pipeline(device.clone(), subpass)?;
        let object_buffer_pool = create_object_buffer_pool(memory_allocator)?;

        let mut object_buffers = ObjectBuffers::new();
        for (&id, object_ref) in object_collection.objects() {
            let object = &*object_ref.as_ref().borrow();
            let buffer = upload_object(&object_buffer_pool, object)
                .context("initial upload object to buffer")?;
            object_buffers.update_or_push(id, buffer);
        }

        let desc_set = create_desc_set(
            descriptor_allocator.borrow(),
            pipeline.clone(),
            object_buffers.buffers(),
        )?;
        Ok(Self {
            descriptor_allocator,
            pipeline,
            object_buffer_pool,
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

    pub fn update_object_buffers(
        &mut self,
        object_collection: &ObjectCollection,
        object_delta: ObjectsDelta,
    ) -> anyhow::Result<()> {
        let mut lowest_changed_index = usize::MAX;

        // freed objects
        for free_id in object_delta.remove {
            if let Some(removed_index) = self.object_buffers.remove(free_id) {
                trace!("removing object buffer id = {}", free_id);
                if removed_index < lowest_changed_index {
                    lowest_changed_index = removed_index;
                }
            } else {
                debug!(
                    "object buffer id = {} was requested to be removed but not found!",
                    free_id
                );
            }
        }

        // added objects
        for set_id in object_delta.update {
            if let Some(object_ref) = object_collection.get(set_id) {
                trace!("adding or updating object buffer id = {}", set_id);
                let object = &*object_ref.as_ref().borrow();
                let buffer = upload_object(&self.object_buffer_pool, object)
                    .context("uploading object data to buffer")?;
                let set_index = self.object_buffers.update_or_push(set_id, buffer);
                if set_index < lowest_changed_index {
                    lowest_changed_index = set_index;
                }
            } else {
                warn!(
                    "requsted update for object id = {} but wasn't found in object collection!",
                    set_id
                );
            }
        }

        // todo bounding box indices

        // update descriptor set
        self.desc_set = create_desc_set(
            self.descriptor_allocator.borrow(),
            self.pipeline.clone(),
            self.object_buffers.buffers(),
        )?;

        Ok(())
    }
}

fn create_object_buffer_pool(
    memory_allocator: Arc<StandardMemoryAllocator>,
) -> anyhow::Result<CpuBufferPool<ObjectDataUnit>> {
    debug!(
        "reserving {} bytes for object buffer pool",
        INIT_BUFFER_POOL_RESERVE
    );
    let object_buffer_pool: CpuBufferPool<ObjectDataUnit> = CpuBufferPool::new(
        memory_allocator,
        BufferUsage {
            storage_buffer: true,
            ..BufferUsage::empty()
        },
        MemoryUsage::Upload,
    );
    object_buffer_pool
        .reserve(INIT_BUFFER_POOL_RESERVE as u64)
        .context("reserving object buffer pool")?;
    Ok(object_buffer_pool)
}

fn upload_object(
    object_buffer_pool: &CpuBufferPool<ObjectDataUnit>,
    object: &Object,
) -> Result<Arc<CpuBufferPoolChunk<ObjectDataUnit>>, AllocationCreationError> {
    trace!("uploading object id = {} to buffer", object.id());
    object_buffer_pool.from_iter(object.encoded_data())
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
    let mut layout_create_infos =
        DescriptorSetLayoutCreateInfo::from_requirements(frag_entry.descriptor_requirements());
    set_object_buffer_variable_descriptor_count(&mut layout_create_infos)?;

    let set_layouts = layout_create_infos
        .into_iter()
        .map(|desc| DescriptorSetLayout::new(device.clone(), desc))
        .collect::<Result<Vec<_>, DescriptorSetLayoutCreationError>>()
        .context("creating scene geometry descriptor set layouts")?;

    PipelineLayout::new(
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
    .context("creating scene geometry pipeline layout")
}

/// We need to update the binding info generated by vulkano to have a variable descriptor count for the object buffers
fn set_object_buffer_variable_descriptor_count(
    layout_create_infos: &mut Vec<DescriptorSetLayoutCreateInfo>,
) -> anyhow::Result<()> {
    let binding = layout_create_infos
        .get_mut(descriptor::SET_BUFFERS)
        .ok_or(CreateDescriptorSetError::InvalidDescriptorSetIndex {
            index: descriptor::SET_BUFFERS,
            shader_path: FRAG_SHADER_PATH,
        })
        .context("missing object buffer descriptor set layout ci for geometry shader")?
        .bindings
        .get_mut(&descriptor::BINDING_OBJECTS)
        .context("missing object buffer descriptor binding for geometry shader")?;
    binding.variable_descriptor_count = true;
    binding.descriptor_count = MAX_OBJECT_BUFFERS;
    Ok(())
}

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
        [WriteDescriptorSet::buffer_array(
            descriptor::BINDING_OBJECTS,
            0,
            object_buffers
                .iter()
                .map(|buffer| buffer.clone() as Arc<dyn BufferAccess>) // probably a nicer way to do this conversion but https://stackoverflow.com/questions/58683548/how-to-coerce-a-vec-of-structs-to-a-vec-of-trait-objects
                .collect::<Vec<Arc<dyn BufferAccess>>>(),
        )],
    )
    .context("creating object buffer desc set")
}

struct ObjectBuffers {
    ids: Vec<ObjectId>,
    buffers: Vec<Arc<CpuBufferPoolChunk<ObjectDataUnit>>>,
}
impl ObjectBuffers {
    pub fn new() -> Self {
        Self {
            ids: Vec::new(),
            buffers: Vec::new(),
        }
    }

    /// Returns the index
    pub fn update_or_push(
        &mut self,
        id: ObjectId,
        buffer: Arc<CpuBufferPoolChunk<ObjectDataUnit>>,
    ) -> usize {
        debug_assert!(self.ids.len() == self.buffers.len());
        if let Some(index) = self.get_index(id) {
            self.buffers[index] = buffer;
            index
        } else {
            self.ids.push(id);
            self.buffers.push(buffer);
            self.ids.len() - 1
        }
    }

    /// Returns the vec index if the id was found and removed.
    pub fn remove(&mut self, id: ObjectId) -> Option<usize> {
        debug_assert!(self.ids.len() == self.buffers.len());
        let index_res = self.get_index(id);
        if let Some(index) = index_res {
            self.ids.remove(index);
            self.buffers.remove(index);
        }
        index_res
    }

    pub fn get_index(&self, id: ObjectId) -> Option<usize> {
        self.ids.iter().position(|&x| x == id)
    }

    pub fn buffers(&self) -> &Vec<Arc<CpuBufferPoolChunk<ObjectDataUnit>>> {
        &self.buffers
    }
}
