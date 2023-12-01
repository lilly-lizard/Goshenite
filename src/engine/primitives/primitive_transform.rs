use crate::renderer::shader_interfaces::primitive_op_buffer::PrimitiveTransformSlice;
use glam::{Mat3, Quat, Vec3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PrimitiveTransform {
    /// Primitive translation relative to object origin
    pub center: Vec3,
    /// Edit this make tentative adjustments to the rotation that can easily be undone
    /// e.g. when dragging a UI element.
    pub rotation_tentative_append: Quat,
    /// Primitive rotation quaternion
    pub rotation: Quat,
}

impl PrimitiveTransform {
    pub const fn new(center: Vec3, rotation: Quat) -> Self {
        Self {
            center,
            rotation,
            ..DEFAULT_PRIMITIVE_TRANSFORM
        }
    }

    pub fn rotation_matrix(&self) -> Mat3 {
        Mat3::from_quat(self.rotation)
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

pub const DEFAULT_PRIMITIVE_TRANSFORM: PrimitiveTransform = PrimitiveTransform {
    center: Vec3::ZERO,
    rotation_tentative_append: Quat::IDENTITY,
    rotation: Quat::IDENTITY,
};

impl Default for PrimitiveTransform {
    fn default() -> Self {
        DEFAULT_PRIMITIVE_TRANSFORM
    }
}
