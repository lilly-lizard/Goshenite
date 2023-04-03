use crate::renderer::shader_interfaces::primitive_op_buffer::PrimitiveTransformSlice;
use glam::{Mat3, Vec3};

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct PrimitiveTransform {
    /// Primitive translation relative to object origin
    pub center: Vec3,
}

impl PrimitiveTransform {
    pub const fn new_default() -> Self {
        Self { center: Vec3::ZERO }
    }

    pub fn rotation_matrix(&self) -> Mat3 {
        Mat3::from_rotation_x(std::f32::consts::TAU / 8.)
    }

    pub fn encoded(&self, parent_origin: Vec3) -> PrimitiveTransformSlice {
        let inverse_rotation_mat = self.rotation_matrix().inverse();
        let rotation_cols_array = inverse_rotation_mat.to_cols_array();

        let center = self.center + parent_origin;
        [
            center.x.to_bits(),
            center.y.to_bits(),
            center.z.to_bits(),
            rotation_cols_array[0].to_bits(),
            rotation_cols_array[1].to_bits(),
            rotation_cols_array[2].to_bits(),
            rotation_cols_array[3].to_bits(),
            rotation_cols_array[4].to_bits(),
            rotation_cols_array[5].to_bits(),
            rotation_cols_array[6].to_bits(),
            rotation_cols_array[7].to_bits(),
            rotation_cols_array[8].to_bits(),
        ]
    }
}
