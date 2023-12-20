use crate::{
    helper::{
        angle::Angle,
        axis::{Axis, AxisRotation},
    },
    renderer::shader_interfaces::primitive_op_buffer::PrimitiveTransformSlice,
};
use glam::{Mat3, Quat, Vec3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PrimitiveTransform {
    /// Primitive translation relative to object origin
    pub center: Vec3,
    /// Edit this make tentative adjustments to the rotation that can easily be undone
    /// e.g. when dragging a UI element.
    rotation_tentative_append: AxisRotation,
    /// Primitive rotation quaternion
    rotation: Quat,
}

impl PrimitiveTransform {
    pub const fn new(center: Vec3, rotation: Quat) -> Self {
        Self {
            center,
            rotation,
            ..Self::DEFAULT
        }
    }

    pub fn total_rotation(&self) -> Quat {
        let rotation_tentative_append_quat = self.rotation_tentative_append.to_quat()
            .expect("Axis::Direction should only be set via the `new_direction()` function to avoid un-normalizable values");
        rotation_tentative_append_quat.mul_quat(self.rotation)
    }

    pub fn rotation_matrix(&self) -> Mat3 {
        Mat3::from_quat(self.total_rotation())
    }

    pub fn gpu_encoded(&self, parent_origin: Vec3) -> PrimitiveTransformSlice {
        let rotation_cols_array = self.rotation_matrix().to_cols_array();

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

    #[inline]
    pub fn rotation_tentative_append(&self) -> AxisRotation {
        self.rotation_tentative_append
    }

    pub fn commit_tentative_rotation(&mut self) {
        self.rotation = self.total_rotation();
        self.rotation_tentative_append = AxisRotation::DEFAULT;
    }

    pub fn set_tentative_rotation(&mut self, new_rotation: AxisRotation) {
        self.rotation_tentative_append = new_rotation;
    }

    pub fn set_tentative_rotation_axis(&mut self, new_axis: Axis) {
        self.rotation_tentative_append.axis = new_axis;
    }

    pub fn set_tentative_rotation_angle(&mut self, new_angle: Angle) {
        self.rotation_tentative_append.angle = new_angle;
    }

    pub fn reset_tentative_rotation(&mut self) {
        self.rotation_tentative_append = AxisRotation::DEFAULT;
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
