use bytemuck::{Pod, Zeroable};
use glam::Vec3;

use crate::engine::object::object::ObjectId;

/// Should match inputs in `overlay.vert`
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Zeroable, Pod)]
pub struct OverlayVertex {
    pub in_position: [f32; 4],
    pub in_normal: [f32; 4],
    pub in_color: [f32; 4],
}
vulkano::impl_vertex!(OverlayVertex, in_position, in_normal, in_color);
impl OverlayVertex {
    pub const fn new(position: Vec3, normal: Vec3, color: Vec3) -> Self {
        Self {
            in_position: [position.x, position.y, position.z, 1.],
            in_normal: [normal.x, normal.y, normal.z, 1.],
            in_color: [color.x, color.y, color.z, 1.],
        }
    }
}

/// Should match vertex definition for `gui.vert` (except color is `[f32; 4]`)
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Zeroable, Pod)]
pub struct EguiVertex {
    pub in_position: [f32; 2],
    pub in_tex_coords: [f32; 2],
    pub in_color: [f32; 4],
}
vulkano::impl_vertex!(EguiVertex, in_position, in_tex_coords, in_color);

/// Should match inputs in `bounding_box.vert`
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Zeroable, Pod)]
pub struct BoundingBoxVertex {
    pub in_position: [f32; 4],
    pub in_object_id: u32,
}
vulkano::impl_vertex!(BoundingBoxVertex, in_position, in_object_id);
impl BoundingBoxVertex {
    pub const fn new(position: Vec3, object_id: ObjectId) -> Self {
        Self {
            in_position: [position.x, position.y, position.z, 1.],
            in_object_id: object_id as u32,
        }
    }
}
