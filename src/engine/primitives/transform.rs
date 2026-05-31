use crate::{
    helper::{
        angle::Angle,
        axis::{Axis, AxisRotation},
    },
    renderer::shader_interfaces::primitive_op_buffer::PrimitiveTransformSlice,
};
use glam::{Mat3, Quat, Vec3};
use serde::{Deserialize, Serialize};

// ~~ Transform ~~

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    /// Primitive translation relative to object origin
    pub translation: Vec3,
    /// Edit this make tentative adjustments to the rotation that can easily be undone
    /// e.g. when dragging a UI element. __TODO__ delete?
    rotation_tentative_append: AxisRotation,
    /// Primitive rotation quaternion
    rotation: Quat,
}

impl Transform {
    pub const fn new(translation: Vec3, rotation: Quat) -> Self {
        Self {
            translation,
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

        let center = self.translation + parent_origin;
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

    pub fn set_tentative_rotation_axis(&mut self, new_axis: Axis) {
        self.rotation_tentative_append.axis = new_axis;
    }

    pub fn set_tentative_rotation_angle(&mut self, new_angle: Angle) {
        self.rotation_tentative_append.angle = new_angle;
    }

    pub const DEFAULT: Transform = Transform {
        translation: Vec3::ZERO,
        rotation_tentative_append: AxisRotation::DEFAULT,
        rotation: Quat::IDENTITY,
    };
}

impl Default for Transform {
    fn default() -> Self {
        Self::DEFAULT
    }
}

// ~~ Object Instancing ~~

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ObjectInstances {
    Single,
    OneDimension {
        instance_count: usize,
        transform: Transform,
    },
    TwoDimension {
        instance_count: [usize; 2],
        transform_a: Transform,
        transform_b: Transform,
    },
}

impl Default for ObjectInstances {
    fn default() -> Self {
        Self::Single
    }
}

impl ObjectInstances {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Single => "Single",
            Self::OneDimension { .. } => "1D",
            Self::TwoDimension { .. } => "2D",
        }
    }

    pub const fn default_1d() -> Self {
        Self::OneDimension {
            instance_count: 2,
            transform: Transform::new(Vec3::new(1., 0., 0.), Quat::IDENTITY),
        }
    }

    pub const fn default_2d() -> Self {
        Self::TwoDimension {
            instance_count: [2, 2],
            transform_a: Transform::new(Vec3::new(1., 0., 0.), Quat::IDENTITY),
            transform_b: Transform::new(Vec3::new(0., 1., 0.), Quat::IDENTITY),
        }
    }

    pub const VARIANTS: [Self; 3] = [Self::Single, Self::default_1d(), Self::default_2d()];
}
