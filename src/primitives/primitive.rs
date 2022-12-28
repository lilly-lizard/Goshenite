use super::{cube::Cube, sphere::Sphere};
use crate::{
    helper::from_enum_impl::from_enum_impl,
    shaders::primitive_buffer::{primitive_codes, PrimitiveDataSlice, PRIMITIVE_UNIT_LEN},
};
use glam::Vec3;

/// A primitive is a basic geometric building block that can be manipulated and combined
/// using [`Operation`]s
pub trait PrimitiveTrait: Default + PartialEq + Clone {
    /// Returns buffer compatible primitive data as a [`PrimitiveDataSlice`].
    ///
    /// _Note: must match the decode process in `scene.comp`_
    fn encode(&self) -> PrimitiveDataSlice;
    /// Returns the spacial center of the primitive.
    fn center(&self) -> Vec3;
    /// Returns the primitive type as a str
    fn type_name(&self) -> &'static str;
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
            Self::Null => [primitive_codes::NULL; PRIMITIVE_UNIT_LEN],
            Self::Sphere(s) => s.encode(),
            Self::Cube(c) => c.encode(),
        }
    }

    fn center(&self) -> Vec3 {
        match self {
            Self::Null => Default::default(),
            Self::Sphere(s) => s.center(),
            Self::Cube(c) => c.center(),
        }
    }

    fn type_name(&self) -> &'static str {
        match self {
            Self::Null => "Null",
            Self::Sphere(s) => s.type_name(),
            Self::Cube(c) => c.type_name(),
        }
    }
}
impl Default for Primitive {
    fn default() -> Self {
        Self::Null
    }
}
from_enum_impl!(Primitive, Sphere);
from_enum_impl!(Primitive, Cube);
