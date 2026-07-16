use super::{cube::Cube, sphere::Sphere, uber_primitive::UberPrimitive};
use crate::{
    engine::{aabb::Aabb, primitives::transform::Transform},
    helper::from_enum_macro::impl_from_for_enum_variant,
};
use serde::{Deserialize, Serialize};

// ~~ Primitive ~~

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Primitive {
    Cube(Cube),
    Sphere(Sphere),
    UberPrimitive(UberPrimitive),
}

impl Primitive {
    pub fn variants_with_names() -> Vec<(Self, &'static str)> {
        Self::VARIANTS
            .iter()
            .map(|primitive| (primitive.clone(), primitive.type_name()))
            .collect()
    }

    pub const VARIANTS: &'static [Primitive] = &[
        Primitive::Cube(Cube::DEFAULT),
        Primitive::Sphere(Sphere::DEFAULT),
        Primitive::UberPrimitive(UberPrimitive::DEFAULT),
    ];
    pub const DEFAULT: Primitive = Primitive::Cube(Cube::DEFAULT);
}

impl Default for Primitive {
    fn default() -> Self {
        Self::DEFAULT
    }
}

macro_rules! primitive_fn_match {
    ($self:ident, $primitive_fn:ident) => {
        match $self {
            Self::Sphere(p) => p.$primitive_fn(),
            Self::Cube(p) => p.$primitive_fn(),
            Self::UberPrimitive(p) => p.$primitive_fn(),
        }
    };
}

impl EncodablePrimitive for Primitive {
    fn type_name(&self) -> &'static str {
        primitive_fn_match!(self, type_name)
    }

    fn uber_s(&self) -> [f32; 4] {
        primitive_fn_match!(self, uber_s)
    }

    fn uber_r(&self) -> [f32; 2] {
        primitive_fn_match!(self, uber_r)
    }

    fn aabb(&self, primitive_transform: Transform) -> Aabb {
        match self {
            Self::Sphere(p) => p.aabb(primitive_transform),
            Self::Cube(p) => p.aabb(primitive_transform),
            Self::UberPrimitive(p) => p.aabb(primitive_transform),
        }
    }
}

impl_from_for_enum_variant!(Primitive, Cube);
impl_from_for_enum_variant!(Primitive, Sphere);
impl_from_for_enum_variant!(Primitive, UberPrimitive);

// ~~ Encodable Primitive ~~

/// Methods required to encode and process primitive data. Mostly for GPU rendering.
pub trait EncodablePrimitive: Send + Sync + Serialize {
    /// Returns the primitive type as a str
    fn type_name(&self) -> &'static str;

    /// Defines the shape in the uber primitive sdf
    fn uber_s(&self) -> [f32; 4];

    /// Defines the shape in the uber primitive sdf
    fn uber_r(&self) -> [f32; 2];

    /// Axis aligned bounding box
    fn aabb(&self, primitive_transform: Transform) -> Aabb;
}
