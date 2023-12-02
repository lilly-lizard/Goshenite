use core::fmt;
use glam::{DQuat, DVec3, Quat, Vec3};

use super::angle::Angle;

// ~~ Cartesian Axis ~~

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CartesianAxis {
    X,
    Y,
    Z,
}

impl CartesianAxis {
    pub fn as_vec3(&self) -> Vec3 {
        match self {
            Self::X => Vec3::X,
            Self::Y => Vec3::Y,
            Self::Z => Vec3::Z,
        }
    }

    pub fn as_dvec3(&self) -> DVec3 {
        match self {
            Self::X => DVec3::X,
            Self::Y => DVec3::Y,
            Self::Z => DVec3::Z,
        }
    }
}

impl Default for CartesianAxis {
    fn default() -> Self {
        Self::X
    }
}

// ~~ Axis ~~

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Axis {
    Cartesian(CartesianAxis),
    /// This value should always be normalized. Recommend using `Self::new_direction` to set this.
    Direction(Vec3),
}

impl Axis {
    /// Normalizes `direction_vec` before returning `Self::Direction`
    pub fn new_direction(direction: Vec3) -> Result<Self, AxisError> {
        let normalized_vec = direction
            .try_normalize()
            .ok_or(AxisError::DirectionCantBeNormalized(direction))?;
        Ok(Self::Direction(normalized_vec))
    }

    pub fn to_vec3(&self) -> Vec3 {
        match self {
            Self::Cartesian(axis) => axis.as_vec3(),
            Self::Direction(dir) => dir.normalize_or_zero(),
        }
    }

    pub fn to_dvec3(&self) -> DVec3 {
        match self {
            Self::Cartesian(axis) => axis.as_dvec3(),
            Self::Direction(dir) => dir.as_dvec3().normalize_or_zero(),
        }
    }

    pub fn to_vec3_normalized(&self) -> Result<Vec3, AxisError> {
        let vec = match self {
            Self::Cartesian(axis) => axis.as_vec3(),
            Self::Direction(dir) => dir
                .try_normalize()
                .ok_or(AxisError::DirectionCantBeNormalized(*dir))?,
        };
        Ok(vec)
    }

    pub fn to_dvec3_normalized(&self) -> Result<DVec3, AxisError> {
        let vec = match self {
            Self::Cartesian(axis) => axis.as_dvec3(),
            Self::Direction(dir) => dir
                .as_dvec3()
                .try_normalize()
                .ok_or(AxisError::DirectionCantBeNormalized(*dir))?,
        };
        Ok(vec)
    }
}

impl Default for Axis {
    fn default() -> Self {
        Self::Cartesian(CartesianAxis::default())
    }
}

// ~~ Axis Rotation ~~

pub const DEFAULT_AXIS_ROTATION: AxisRotation = AxisRotation {
    axis: Axis::Cartesian(CartesianAxis::X),
    angle: Angle::ZERO,
};

/// Describes rotation around an axis
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct AxisRotation {
    pub axis: Axis,
    pub angle: Angle,
}

impl AxisRotation {
    pub fn to_quat(&self) -> Result<Quat, AxisError> {
        let axis = self.axis.to_vec3_normalized()?;
        let angle = self.angle.to_radians() as f32;
        Ok(Quat::from_axis_angle(axis, angle))
    }

    pub fn to_dquat(&self) -> Result<DQuat, AxisError> {
        let axis = self.axis.to_dvec3_normalized()?;
        let angle = self.angle.to_radians();
        Ok(DQuat::from_axis_angle(axis, angle))
    }
}

// ~~ Axis Error ~~

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AxisError {
    DirectionCantBeNormalized(Vec3),
}

impl std::fmt::Display for AxisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DirectionCantBeNormalized(vec) => {
                write!(
                    f,
                    "axis direction vector {} cannot be normalized (length is close to 0)",
                    vec
                )
            }
        }
    }
}

impl std::error::Error for AxisError {}
