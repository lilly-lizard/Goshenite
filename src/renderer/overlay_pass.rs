use super::common::{create_shader_module, CreateShaderError};
use crate::{
    camera::Camera,
    primitives::{primitive::PrimitiveTrait, primitive_collection::PrimitiveCollection},
    shaders::shader_interfaces::{OverlayPushConstants, OverlayVertex, SHADER_ENTRY_POINT},
};
use anyhow::Context;
use glam::{Vec3, Vec4};
use std::sync::Arc;
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    command_buffer::AutoCommandBufferBuilder,
    device::Device,
    pipeline::{
        graphics::{
            color_blend::ColorBlendState,
            input_assembly::{InputAssemblyState, PrimitiveTopology},
            rasterization::{CullMode, FrontFace, RasterizationState},
            vertex_input::BuffersDefinition,
            viewport::{Viewport, ViewportState},
        },
        GraphicsPipeline, Pipeline,
    },
    render_pass::Subpass,
};

const VERT_SHADER_PATH: &str = "assets/shader_binaries/overlay.vert.spv";
const FRAG_SHADER_PATH: &str = "assets/shader_binaries/overlay.frag.spv";

pub struct OverlayPass {
    pipeline: Arc<GraphicsPipeline>,
    vertex_buffer: Arc<CpuAccessibleBuffer<[OverlayVertex]>>,
}
// Public functions
impl OverlayPass {
    pub fn new(device: Arc<Device>, subpass: Subpass) -> anyhow::Result<Self> {
        let pipeline = create_pipeline(device.clone(), subpass)?;
        let vertex_buffer = create_vertex_buffer(device.clone())?;
        Ok(Self {
            pipeline,
            vertex_buffer,
        })
    }

    // todo doc
    pub fn record_commands<L>(
        &mut self,
        command_buffer: &mut AutoCommandBufferBuilder<L>,
        camera: &Camera,
        primitive_collection: &PrimitiveCollection,
        viewport: Viewport,
    ) -> anyhow::Result<()> {
        // if a primitive is selected, render the xyz coordinate indicator at its center
        if let Some(primitive) = primitive_collection.selected_primitive() {
            let push_constants = OverlayPushConstants::new(
                camera.proj_matrix() * camera.view_matrix(),
                Vec4::from((primitive.center(), 0.0)),
            );
            command_buffer
                .bind_pipeline_graphics(self.pipeline.clone())
                .set_viewport(0, [viewport])
                .push_constants(self.pipeline.layout().clone(), 0, push_constants)
                .bind_vertex_buffers(0, self.vertex_buffer.clone())
                .draw(VERTEX_COUNT as u32, 1, 0, 0)
                .context("recording overlay draw commands")?;
        }
        Ok(())
    }
}

fn create_pipeline(device: Arc<Device>, subpass: Subpass) -> anyhow::Result<Arc<GraphicsPipeline>> {
    let vert_module = create_shader_module(device.clone(), VERT_SHADER_PATH)?;
    let vert_shader =
        vert_module
            .entry_point(SHADER_ENTRY_POINT)
            .ok_or(CreateShaderError::MissingEntryPoint(
                VERT_SHADER_PATH.to_string(),
            ))?;
    let frag_module = create_shader_module(device.clone(), FRAG_SHADER_PATH)?;
    let frag_shader =
        frag_module
            .entry_point(SHADER_ENTRY_POINT)
            .ok_or(CreateShaderError::MissingEntryPoint(
                FRAG_SHADER_PATH.to_string(),
            ))?;
    GraphicsPipeline::start()
        .vertex_input_state(BuffersDefinition::new().vertex::<OverlayVertex>())
        .vertex_shader(vert_shader, ())
        .input_assembly_state(InputAssemblyState::new().topology(PrimitiveTopology::TriangleList))
        .fragment_shader(frag_shader, ())
        .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
        .color_blend_state(ColorBlendState::new(1))
        .rasterization_state(
            RasterizationState::new()
                .cull_mode(CullMode::Back)
                .front_face(FrontFace::CounterClockwise),
        )
        .render_pass(subpass)
        .build(device.clone())
        .context("creating overlay pipeline")
}

fn create_vertex_buffer(
    device: Arc<Device>,
) -> anyhow::Result<Arc<CpuAccessibleBuffer<[OverlayVertex]>>> {
    CpuAccessibleBuffer::from_iter(
        device.clone(),
        BufferUsage {
            vertex_buffer: true,
            ..BufferUsage::empty()
        },
        false,
        VERTICES,
    )
    .context("creating overlay vertex buffer")
}

/// Color of the x axis indicator
const X_COLOR: Vec3 = Vec3::new(0.8, 0.0, 0.0);
/// Line length
const L_LEN: f32 = 0.5;
/// Line thickness
const L_THI: f32 = 0.05;
const VERTEX_COUNT: usize = 30;
// counter-clockwise front face
const VERTICES: [OverlayVertex; VERTEX_COUNT] = [
    // x axis indicator (red)
    // face x t1
    OverlayVertex::new(Vec3::new(L_LEN, -L_THI, -L_THI), Vec3::X, X_COLOR),
    OverlayVertex::new(Vec3::new(L_LEN, L_THI, -L_THI), Vec3::X, X_COLOR),
    OverlayVertex::new(Vec3::new(L_LEN, -L_THI, L_THI), Vec3::X, X_COLOR),
    // face x t2
    OverlayVertex::new(Vec3::new(L_LEN, L_THI, L_THI), Vec3::X, X_COLOR),
    OverlayVertex::new(Vec3::new(L_LEN, -L_THI, L_THI), Vec3::X, X_COLOR),
    OverlayVertex::new(Vec3::new(L_LEN, L_THI, -L_THI), Vec3::X, X_COLOR),
    // face ny t1
    OverlayVertex::new(Vec3::new(-L_THI, -L_THI, -L_THI), Vec3::NEG_Y, X_COLOR),
    OverlayVertex::new(Vec3::new(L_LEN, -L_THI, -L_THI), Vec3::NEG_Y, X_COLOR),
    OverlayVertex::new(Vec3::new(L_THI, -L_THI, L_THI), Vec3::NEG_Y, X_COLOR),
    // face ny t2
    OverlayVertex::new(Vec3::new(L_THI, -L_THI, L_THI), Vec3::NEG_Y, X_COLOR),
    OverlayVertex::new(Vec3::new(L_LEN, -L_THI, -L_THI), Vec3::NEG_Y, X_COLOR),
    OverlayVertex::new(Vec3::new(L_LEN, -L_THI, L_THI), Vec3::NEG_Y, X_COLOR),
    // face nz t1
    OverlayVertex::new(Vec3::new(-L_THI, -L_THI, -L_THI), Vec3::NEG_Z, X_COLOR),
    OverlayVertex::new(Vec3::new(L_THI, L_THI, -L_THI), Vec3::NEG_Z, X_COLOR),
    OverlayVertex::new(Vec3::new(L_LEN, -L_THI, -L_THI), Vec3::NEG_Z, X_COLOR),
    // face nz t2
    OverlayVertex::new(Vec3::new(L_THI, L_THI, -L_THI), Vec3::NEG_Z, X_COLOR),
    OverlayVertex::new(Vec3::new(L_LEN, L_THI, -L_THI), Vec3::NEG_Z, X_COLOR),
    OverlayVertex::new(Vec3::new(L_LEN, -L_THI, -L_THI), Vec3::NEG_Z, X_COLOR),
    // face y t1
    OverlayVertex::new(Vec3::new(L_THI, L_THI, L_THI), Vec3::Y, X_COLOR),
    OverlayVertex::new(Vec3::new(L_LEN, L_THI, L_THI), Vec3::Y, X_COLOR),
    OverlayVertex::new(Vec3::new(L_LEN, L_THI, -L_THI), Vec3::Y, X_COLOR),
    // face y t2
    OverlayVertex::new(Vec3::new(L_LEN, L_THI, -L_THI), Vec3::Y, X_COLOR),
    OverlayVertex::new(Vec3::new(L_THI, L_THI, -L_THI), Vec3::Y, X_COLOR),
    OverlayVertex::new(Vec3::new(L_THI, L_THI, L_THI), Vec3::Y, X_COLOR),
    // face z t1
    OverlayVertex::new(Vec3::new(L_THI, -L_THI, L_THI), Vec3::Z, X_COLOR),
    OverlayVertex::new(Vec3::new(L_LEN, -L_THI, L_THI), Vec3::Z, X_COLOR),
    OverlayVertex::new(Vec3::new(L_LEN, L_THI, L_THI), Vec3::Z, X_COLOR),
    // face z t2
    OverlayVertex::new(Vec3::new(L_LEN, L_THI, L_THI), Vec3::Z, X_COLOR),
    OverlayVertex::new(Vec3::new(L_THI, L_THI, L_THI), Vec3::Z, X_COLOR),
    OverlayVertex::new(Vec3::new(L_THI, -L_THI, L_THI), Vec3::Z, X_COLOR),
];
