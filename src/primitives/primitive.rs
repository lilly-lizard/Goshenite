use super::{cube::Cube, sphere::Sphere};
use crate::{
    helper::from_enum_impl::from_enum_impl,
    shaders::primitive_buffer::{primitive_codes, PrimitiveDataSlice, PRIMITIVE_UNIT_LEN},
};
use glam::Vec3;

/// Required functions for a usable primitive.
pub trait PrimitiveTrait: Default + PartialEq + Clone {
    /// Returns the primitive data encoded as a [`PrimitiveDataSlice`].
    ///
    /// _Note: must match the decode process in `scene.comp`_
    fn encode(&self) -> PrimitiveDataSlice;
    /// Returns the spacial center of the primitive.
    fn center(&self) -> Vec3;
}

/// Enum of all the supported primitive types.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Primitive {
    Null,
    Sphere(Sphere),
    Cube(Cube),
}
impl PrimitiveTrait for Primitive {
    fn encode(&self) -> PrimitiveDataSlice {
        match self {
            Primitive::Null => [primitive_codes::NULL; PRIMITIVE_UNIT_LEN],
            Primitive::Sphere(s) => s.encode(),
            Primitive::Cube(c) => c.encode(),
        }
    }
    fn center(&self) -> Vec3 {
        match self {
            Primitive::Null => Default::default(),
            Primitive::Sphere(s) => s.center(),
            Primitive::Cube(c) => c.center(),
        }
    }
}
impl Default for Primitive {
    fn default() -> Self {
        Self::Null
    }
}
impl Primitive {
    /// Returns the name of the enum primitive type
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Null => "Null",
            Self::Sphere(_) => "Sphere",
            Self::Cube(_) => "Cube",
        }
    }
}
from_enum_impl!(Primitive, Sphere);
from_enum_impl!(Primitive, Cube);
