use crate::{engine::object::object::ObjectId, helper::unique_id_gen::UniqueIdType};
use bytemuck::NoUninit;
use glam::Vec3;

/// Should match vertex definition for `gui.vert` (except color is `[f32; 4]`)
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, NoUninit)]
pub struct EguiVertex {
    pub in_position: [f32; 2],
    pub in_tex_coords: [f32; 2],
    pub in_color: [f32; 4],
}

impl EguiVertex {
    pub fn from_egui_vertex(egui_vertex: &egui::epaint::Vertex) -> Self {
        let color = [
            egui_vertex.color.r() as f32 / 255.,
            egui_vertex.color.g() as f32 / 255.,
            egui_vertex.color.b() as f32 / 255.,
            egui_vertex.color.a() as f32 / 255.,
        ];

        Self {
            in_position: egui_vertex.pos.into(),
            in_tex_coords: egui_vertex.uv.into(),
            in_color: color,
        }
    }
}

/// Should match inputs in `bounding_mesh.vert`
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, NoUninit)]
pub struct BoundingBoxVertex {
    pub in_position: [f32; 4],
    pub in_object_id: u32,
}

impl BoundingBoxVertex {
    pub fn new(position: Vec3, object_id: ObjectId) -> Self {
        Self {
            in_position: [position.x, position.y, position.z, 1.],
            in_object_id: object_id.raw_id() as u32,
        }
    }
}

/// Should match inputs in `gizmos.frag`
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, NoUninit)]
pub struct GizmoVertex {
    pub in_position: [f32; 4],
}
