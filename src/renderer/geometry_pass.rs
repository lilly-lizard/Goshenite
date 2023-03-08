use super::{
    config_renderer::SHADER_ENTRY_POINT,
    object_buffers::ObjectBuffers,
    shader_interfaces::{
        primitive_op_buffer::PrimitiveOpBufferUnit, uniform_buffers::CameraUniformBuffer,
        vertex_inputs::BoundingBoxVertex,
    },
};
use crate::engine::object::{object_collection::ObjectCollection, objects_delta::ObjectsDelta};
use anyhow::Context;
use ash::vk;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::{borrow::Borrow, sync::Arc};

const MAX_OBJECT_BUFFERS: u32 = 256;

const VERT_SHADER_PATH: &str = "assets/shader_binaries/bounding_box.vert.spv";
const FRAG_SHADER_PATH: &str = "assets/shader_binaries/scene_geometry.frag.spv";

// descriptor set and binding indices
mod descriptor {
    pub const SET_CAMERA: usize = 0;
    pub const BINDING_CAMERA: u32 = 0;

    pub const SET_PRIMITIVE_OPS: usize = 1;
    pub const BINDING_PRIMITIVE_OPS: u32 = 0;
}

/// Render the scene geometry and write to g-buffers
pub struct GeometryPass {
    pipeline: Arc<GraphicsPipeline>,

    descriptor_allocator: Arc<StandardDescriptorSetAllocator>,
    /// None indicates there are no object buffers to be bound
    desc_set_primitive_ops: Option<Arc<PersistentDescriptorSet>>,

    object_buffers: ObjectBuffers,
}

// Public functions
impl GeometryPass {
    pub fn new(
        device: Arc<Device>,
        memory_allocator: Arc<StandardMemoryAllocator>,
        descriptor_allocator: Arc<StandardDescriptorSetAllocator>,
        subpass: Subpass,
    ) -> anyhow::Result<Self> {
        let pipeline = create_pipeline(device.clone(), subpass)?;

        let object_buffers = ObjectBuffers::new(memory_allocator)?;

        Ok(Self {
            pipeline,
            descriptor_allocator,
            desc_set_primitive_ops: None,
            object_buffers,
        })
    }

    pub fn update_object_buffers(
        &mut self,
        object_collection: &ObjectCollection,
        object_delta: ObjectsDelta,
    ) -> anyhow::Result<()> {
        // freed objects
        for free_id in object_delta.remove {
            if let Some(_removed_index) = self.object_buffers.remove(free_id) {
                trace!("removing object buffer id = {}", free_id);
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
                self.object_buffers.update_or_push(object)?;
            } else {
                warn!(
                    "requsted update for object id = {} but wasn't found in object collection!",
                    set_id
                );
            }
        }

        // update descriptor set
        self.desc_set_primitive_ops = create_desc_set_primitive_ops(
            self.descriptor_allocator.borrow(),
            self.pipeline.clone(),
            self.object_buffers.primitive_op_buffers(),
        )?;

        Ok(())
    }

    pub fn record_commands<L>(
        &self,
        command_buffer: &mut AutoCommandBufferBuilder<L>,
        viewport: Viewport,
        camera_buffer: Arc<CpuAccessibleBuffer<CameraUniformBuffer>>,
    ) -> anyhow::Result<()> {
        let desc_set_primitive_ops = match &self.desc_set_primitive_ops {
            Some(s) => s.clone(),
            None => {
                trace!("no object buffers found. skipping geometry pass commands...");
                return Ok(());
            }
        };

        let desc_set_camera = create_desc_set_camera(
            &self.descriptor_allocator,
            self.pipeline.clone(),
            camera_buffer,
        )?;

        let desc_sets = vec![desc_set_camera, desc_set_primitive_ops];

        command_buffer
            .set_viewport(0, [viewport])
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                desc_sets,
            );
        self.object_buffers
            .draw_commands(command_buffer, self.pipeline.clone())?;

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

    let pipeline_layout = create_pipeline_layout(device.clone(), &vert_shader, &frag_shader)
        .context("creating geometry pipeline layout")?;

    Ok(GraphicsPipeline::start()
        .render_pass(subpass)
        .vertex_input_state(BuffersDefinition::new().vertex::<BoundingBoxVertex>())
        .input_assembly_state(InputAssemblyState::new().topology(PrimitiveTopology::TriangleList))
        .vertex_shader(vert_shader, ())
        .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
        .rasterization_state(
            RasterizationState::new()
                .cull_mode(CullMode::Back)
                .front_face(FrontFace::CounterClockwise),
        )
        .depth_stencil_state(DepthStencilState::simple_depth_test())
        .fragment_shader(frag_shader, ())
        .with_pipeline_layout(device.clone(), pipeline_layout)
        .context("creating geometry pass pipeline")?)
}

fn create_pipeline_layout(
    device: Arc<Device>,
    vert_entry: &EntryPoint,
    frag_entry: &EntryPoint,
) -> anyhow::Result<Arc<PipelineLayout>> {
    // yeah it's gross. will be moving to ash at some point anyway...
    let desc_requirements = frag_entry
        .descriptor_requirements()
        .map(|((set_f, binding_f), reqs_f)| {
            let mut ret = ((set_f, binding_f), reqs_f.clone());
            for ((set_v, binding_v), reqs_v) in vert_entry.descriptor_requirements() {
                if set_v == set_f && binding_v == binding_f {
                    if let Ok(reqs_vf) = reqs_v.intersection(reqs_f) {
                        ret = ((set_f, binding_f), reqs_vf);
                        break;
                    }
                }
            }
            ret
        })
        .collect::<Vec<((u32, u32), DescriptorRequirements)>>();
    let desc_requirements = desc_requirements.iter().map(|(k, v)| (*k, v));

    let mut layout_create_infos =
        DescriptorSetLayoutCreateInfo::from_requirements(desc_requirements);
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
            push_constant_ranges: vert_entry
                .push_constant_requirements()
                .cloned()
                .into_iter()
                .collect(),
            ..Default::default()
        },
    )
    .context("creating scene geometry pipeline layout")
}

fn create_desc_set_camera(
    descriptor_allocator: &StandardDescriptorSetAllocator,
    pipeline: Arc<GraphicsPipeline>,
    camera_buffer: Arc<CpuAccessibleBuffer<CameraUniformBuffer>>,
) -> anyhow::Result<Arc<PersistentDescriptorSet>> {
    let set_layout = pipeline
        .layout()
        .set_layouts()
        .get(descriptor::SET_CAMERA)
        .ok_or(CreateDescriptorSetError::InvalidDescriptorSetIndex {
            index: descriptor::SET_CAMERA,
            shader_path: FRAG_SHADER_PATH,
        })?
        .to_owned();

    PersistentDescriptorSet::new(
        descriptor_allocator,
        set_layout,
        [WriteDescriptorSet::buffer(
            descriptor::BINDING_CAMERA,
            camera_buffer,
        )],
    )
    .context("creating geometry pass camera desc set")
}

fn create_desc_set_primitive_ops(
    descriptor_allocator: &StandardDescriptorSetAllocator,
    pipeline: Arc<GraphicsPipeline>,
    primitive_op_buffers: &Vec<Arc<CpuBufferPoolChunk<PrimitiveOpBufferUnit>>>,
) -> anyhow::Result<Option<Arc<PersistentDescriptorSet>>> {
    if primitive_op_buffers.is_empty() {
        // descriptor set creation fails when element count is 0
        return Ok(None);
    }

    let set_layout = pipeline
        .layout()
        .set_layouts()
        .get(descriptor::SET_PRIMITIVE_OPS)
        .ok_or(CreateDescriptorSetError::InvalidDescriptorSetIndex {
            index: descriptor::SET_PRIMITIVE_OPS,
            shader_path: FRAG_SHADER_PATH,
        })
        .context("creating primitive op buffer desc set")?
        .to_owned();

    let desc_set = PersistentDescriptorSet::new_variable(
        descriptor_allocator,
        set_layout,
        primitive_op_buffers.len() as u32,
        [WriteDescriptorSet::buffer_array(
            descriptor::BINDING_PRIMITIVE_OPS,
            0,
            primitive_op_buffers
                .iter()
                .map(|buffer| buffer.clone() as Arc<dyn BufferAccess>) // probably a nicer way to do this conversion but https://stackoverflow.com/questions/58683548/how-to-coerce-a-vec-of-structs-to-a-vec-of-trait-objects
                .collect::<Vec<Arc<dyn BufferAccess>>>(),
        )],
    )
    .context("creating primitive op buffer desc set")?;
    Ok(Some(desc_set))
}

/// We need to update the binding info generated by vulkano to have a variable descriptor count for the object buffers
fn set_primitive_op_buffer_variable_descriptor_count(
    layout_create_infos: &mut Vec<DescriptorSetLayoutCreateInfo>,
) -> anyhow::Result<()> {
    let binding = layout_create_infos
        .get_mut(descriptor::SET_PRIMITIVE_OPS)
        .ok_or(CreateDescriptorSetError::InvalidDescriptorSetIndex {
            index: descriptor::SET_PRIMITIVE_OPS,
            shader_path: FRAG_SHADER_PATH,
        })
        .context("missing primitive_op buffer descriptor set layout ci for geometry shader")?
        .bindings
        .get_mut(&descriptor::BINDING_PRIMITIVE_OPS)
        .context("missing primitive_op buffer descriptor binding for geometry shader")?;
    binding.variable_descriptor_count = true;
    binding.descriptor_count = MAX_OBJECT_BUFFERS;

    Ok(())
}
