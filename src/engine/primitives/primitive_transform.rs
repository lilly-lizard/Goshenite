use crate::{
    helper::axis::AxisRotation,
    renderer::shader_interfaces::primitive_op_buffer::PrimitiveTransformSlice,
};
use glam::{Mat3, Quat, Vec3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PrimitiveTransform {
    /// Primitive translation relative to object origin
    pub center: Vec3,
    /// Edit this make tentative adjustments to the rotation that can easily be undone
    /// e.g. when dragging a UI element.
    pub rotation_tentative_append: AxisRotation,
    /// Primitive rotation quaternion
    pub rotation: Quat,
}

impl PrimitiveTransform {
    pub const fn new(center: Vec3, rotation: Quat) -> Self {
        Self {
            center,
            rotation,
            ..Self::DEFAULT
        }
    }

    #[inline]
    pub fn total_rotation(&self) -> Quat {
        let rotation_tentative_append_quat = self.rotation_tentative_append.to_quat()
            .expect("Axis::Direction should only be set via the `new_direction()` function to avoid un-normalizable values");
        rotation_tentative_append_quat.mul_quat(self.rotation)
    }

    pub fn commit_tentative_rotation(&mut self) {
        self.rotation = self.total_rotation();
        self.rotation_tentative_append = AxisRotation::default();
    }

    #[inline]
    pub fn rotation_matrix(&self) -> Mat3 {
        Mat3::from_quat(self.total_rotation())
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

    pub const DEFAULT: PrimitiveTransform = PrimitiveTransform {
        center: Vec3::ZERO,
        rotation_tentative_append: AxisRotation::DEFAULT,
        rotation: Quat::IDENTITY,
    };
}

impl Default for PrimitiveTransform {
    fn default() -> Self {
        Self::DEFAULT
    }
}
