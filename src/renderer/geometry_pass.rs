use super::{
    renderer_config::SHADER_ENTRY_POINT,
    shader_interfaces::{
        primitive_op_buffer::{PrimitiveOpBufferUnit, PRIMITIVE_OP_UNIT_LEN},
        push_constants::CameraPushConstants,
        vertex_inputs::BoundingBoxVertex,
    },
    vulkan_helper::{create_shader_module, CreateDescriptorSetError, CreateShaderError},
};
use crate::engine::{
    aabb::AABB_VERTEX_COUNT,
    object::{
        object::{Object, ObjectId},
        object_collection::ObjectCollection,
        objects_delta::ObjectsDelta,
    },
};
use anyhow::Context;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::{borrow::Borrow, mem::size_of, sync::Arc};
use vulkano::{
    buffer::{
        cpu_pool::CpuBufferPoolChunk, BufferAccess, BufferUsage, CpuAccessibleBuffer, CpuBufferPool,
    },
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
        graphics::{
            input_assembly::{InputAssemblyState, PrimitiveTopology},
            rasterization::{CullMode, FrontFace, RasterizationState},
            vertex_input::BuffersDefinition,
            viewport::{Viewport, ViewportState},
        },
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

/// Reserve for 1024 operations
const INIT_PRIMITIVE_OP_POOL_RESERVE: DeviceSize =
    (1024 * PRIMITIVE_OP_UNIT_LEN * size_of::<PrimitiveOpBufferUnit>()) as DeviceSize;

/// Reserve for 16 AABBs
const INIT_BOUNDING_BOX_POOL_RESERVE: DeviceSize =
    (16 * AABB_VERTEX_COUNT * size_of::<BoundingBoxVertex>()) as DeviceSize;

struct ObjectBuffers {
    bounding_box_buffer_pool: CpuBufferPool<BoundingBoxVertex>,
    primitive_op_buffer_pool: CpuBufferPool<PrimitiveOpBufferUnit>,

    ids: Vec<ObjectId>,
    bounding_boxes: Vec<Arc<CpuBufferPoolChunk<BoundingBoxVertex>>>,
    primitive_ops: Vec<Arc<CpuBufferPoolChunk<PrimitiveOpBufferUnit>>>,
}
impl ObjectBuffers {
    pub fn new(memory_allocator: Arc<StandardMemoryAllocator>) -> anyhow::Result<Self> {
        let bounding_box_buffer_pool = create_bounding_box_buffer_pool(memory_allocator)?;
        let primitive_op_buffer_pool = create_primitive_op_buffer_pool(memory_allocator)?;

        Ok(Self {
            bounding_box_buffer_pool,
            primitive_op_buffer_pool,
            ids: Vec::new(),
            bounding_boxes: Vec::new(),
            primitive_ops: Vec::new(),
        })
    }

    /// Returns the index
    pub fn update_or_push(&mut self, object: &Object) -> anyhow::Result<usize> {
        debug_assert!(self.ids.len() == self.primitive_ops.len());
        debug_assert!(self.ids.len() == self.bounding_boxes.len());

        let id = object.id();

        let primitive_ops_buffer = upload_primitive_ops(&self.primitive_op_buffer_pool, object)
            .context("initial upload object to buffer")?;

        if let Some(index) = self.get_index(id) {
            let bounding_box_buffer =
                upload_bounding_box(&self.bounding_box_buffer_pool, object, index as u32)?;

            self.bounding_boxes[index] = bounding_box_buffer;
            self.primitive_ops[index] = primitive_ops_buffer;

            Ok(index)
        } else {
            let index = self.ids.len();
            let bounding_box_buffer =
                upload_bounding_box(&self.bounding_box_buffer_pool, object, index as u32)?;

            self.ids.push(id);
            self.bounding_boxes.push(bounding_box_buffer);
            self.primitive_ops.push(primitive_ops_buffer);

            Ok(index)
        }
    }

    /// Returns the vec index if the id was found and removed.
    pub fn remove(&mut self, id: ObjectId) -> Option<usize> {
        debug_assert!(self.ids.len() == self.primitive_ops.len());
        let index_res = self.get_index(id);
        if let Some(index) = index_res {
            self.ids.remove(index);
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
}

/// Render the scene geometry and write to g-buffers
pub struct GeometryPass {
    descriptor_allocator: Arc<StandardDescriptorSetAllocator>,
    pipeline: Arc<GraphicsPipeline>,

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

        let mut object_buffers = ObjectBuffers::new(memory_allocator)?;
        for (_id, object_ref) in object_collection.objects() {
            let object = &*object_ref.as_ref().borrow();
            object_buffers.update_or_push(object);
        }

        let desc_set = create_desc_set(
            descriptor_allocator.borrow(),
            pipeline.clone(),
            object_buffers.primitive_op_buffers(),
        )?;

        Ok(Self {
            descriptor_allocator,
            pipeline,
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
                let set_index = self.object_buffers.update_or_push(object)?;
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

        todo!("bounding box indices");

        // update descriptor set
        self.desc_set = create_desc_set(
            self.descriptor_allocator.borrow(),
            self.pipeline.clone(),
            self.object_buffers.primitive_op_buffers(),
        )?;

        Ok(())
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
    object_index: u32,
) -> Result<Arc<CpuBufferPoolChunk<BoundingBoxVertex>>, AllocationCreationError> {
    trace!(
        "uploading bounding box vertices for object id = {} to gpu buffer",
        object.id()
    );
    bounding_box_buffer_pool.from_iter(object.aabb().vertices(object_index))
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
        .vertex_input_state(BuffersDefinition::new().vertex::<BoundingBoxVertex>())
        .input_assembly_state(InputAssemblyState::new().topology(PrimitiveTopology::TriangleList))
        .vertex_shader(vert_shader, ())
        .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
        .rasterization_state(
            RasterizationState::new()
                .cull_mode(CullMode::Back)
                .front_face(FrontFace::CounterClockwise),
        )
        .fragment_shader(frag_shader, ())
        // todo .color_blend_state(ColorBlendState::new(1))
        .render_pass(subpass)
        .with_pipeline_layout(device.clone(), pipeline_layout)
        .context("creating geometry pass pipeline")?)
}

fn create_pipeline_layout(
    device: Arc<Device>,
    frag_entry: &EntryPoint,
) -> anyhow::Result<Arc<PipelineLayout>> {
    let mut layout_create_infos =
        DescriptorSetLayoutCreateInfo::from_requirements(frag_entry.descriptor_requirements());
    set_primitive_op_buffer_variable_descriptor_count(&mut layout_create_infos)?;

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
fn set_primitive_op_buffer_variable_descriptor_count(
    layout_create_infos: &mut Vec<DescriptorSetLayoutCreateInfo>,
) -> anyhow::Result<()> {
    let binding = layout_create_infos
        .get_mut(descriptor::SET_BUFFERS)
        .ok_or(CreateDescriptorSetError::InvalidDescriptorSetIndex {
            index: descriptor::SET_BUFFERS,
            shader_path: FRAG_SHADER_PATH,
        })
        .context("missing primitive_op buffer descriptor set layout ci for geometry shader")?
        .bindings
        .get_mut(&descriptor::BINDING_OBJECTS)
        .context("missing primitive_op buffer descriptor binding for geometry shader")?;
    binding.variable_descriptor_count = true;
    binding.descriptor_count = MAX_OBJECT_BUFFERS;

    Ok(())
}

fn create_desc_set(
    descriptor_allocator: &StandardDescriptorSetAllocator,
    geometry_pipeline: Arc<GraphicsPipeline>,
    primitive_op_buffers: &Vec<Arc<CpuBufferPoolChunk<PrimitiveOpBufferUnit>>>,
) -> anyhow::Result<Arc<PersistentDescriptorSet>> {
    let set_layout = geometry_pipeline
        .layout()
        .set_layouts()
        .get(descriptor::SET_BUFFERS)
        .ok_or(CreateDescriptorSetError::InvalidDescriptorSetIndex {
            index: descriptor::SET_BUFFERS,
            shader_path: FRAG_SHADER_PATH,
        })
        .context("creating primitive op buffer desc set")?
        .to_owned();

    PersistentDescriptorSet::new_variable(
        descriptor_allocator,
        set_layout,
        primitive_op_buffers.len() as u32,
        [WriteDescriptorSet::buffer_array(
            descriptor::BINDING_OBJECTS,
            0,
            primitive_op_buffers
                .iter()
                .map(|buffer| buffer.clone() as Arc<dyn BufferAccess>) // probably a nicer way to do this conversion but https://stackoverflow.com/questions/58683548/how-to-coerce-a-vec-of-structs-to-a-vec-of-trait-objects
                .collect::<Vec<Arc<dyn BufferAccess>>>(),
        )],
    )
    .context("creating primitive op buffer desc set")
}
