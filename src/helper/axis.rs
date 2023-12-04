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

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::X => "X",
            Self::Y => "Y",
            Self::Z => "Z",
        }
    }

    pub fn variants_with_names() -> Vec<(Self, &'static str)> {
        Self::VARIANTS
            .iter()
            .map(|axis| (*axis, axis.as_str()))
            .collect()
    }

    pub const VARIANTS: &[CartesianAxis] = &[CartesianAxis::X, CartesianAxis::Y, CartesianAxis::Z];
    pub const DEFAULT: Self = Self::X;
}

impl Default for CartesianAxis {
    fn default() -> Self {
        Self::DEFAULT
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

    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Cartesian(_) => Self::CARTESIAN_VARIANT_NAME,
            Self::Direction(_) => Self::DIRECTION_VARIANT_NAME,
        }
    }

    pub const CARTESIAN_VARIANT_NAME: &str = "Cartesian";
    pub const DIRECTION_VARIANT_NAME: &str = "Direction";

    pub const DEFAULT_CARTESIAN: Self = Self::Cartesian(CartesianAxis::DEFAULT);
    pub const DEFAULT_DIRECION: Self = Self::Direction(Vec3::X);
}

impl Default for Axis {
    fn default() -> Self {
        Self::DEFAULT_CARTESIAN
    }
}

// ~~ Axis Rotation ~~

/// Describes rotation around an axis
#[derive(Clone, Copy, Debug, PartialEq)]
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

    pub const DEFAULT: AxisRotation = AxisRotation {
        axis: Axis::DEFAULT_CARTESIAN,
        angle: Angle::ZERO,
    };
}

impl Default for AxisRotation {
    fn default() -> Self {
        Self::DEFAULT
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
