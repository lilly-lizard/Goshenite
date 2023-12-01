use core::fmt;
use glam::{DVec3, Vec3};

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
    /// This value must always be normalized. Recommend using `Self::new_direction` to set this.
    Direction(Vec3),
}

impl Axis {
    /// Normalizes `direction_vec` before returning `Self::Direction`
    pub fn new_direction(direction_vec: Vec3) -> Result<Self, AxisError> {
        let normalized_vec = match direction_vec.try_normalize() {
            Some(vec) => vec,
            None => return Err(AxisError::DirectionCantBeNormalized(direction_vec)),
        };
        Ok(Self::Direction(normalized_vec))
    }

    pub fn as_vec3(&self) -> Vec3 {
        match self {
            Self::Cartesian(axis) => axis.as_vec3(),
            Self::Direction(dir) => *dir,
        }
    }

    pub fn as_dvec3(&self) -> DVec3 {
        match self {
            Self::Cartesian(axis) => axis.as_dvec3(),
            Self::Direction(dir) => dir.as_dvec3(),
        }
    }
}

impl Default for Axis {
    fn default() -> Self {
        Self::Cartesian(CartesianAxis::default())
    }
}

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
