use super::{
    cube::{Cube, DEFAULT_CUBE},
    primitive_transform::PrimitiveTransform,
    sphere::{Sphere, DEFAULT_SPHERE},
    uber_primitive::{UberPrimitive, DEFAULT_UBER_PRIMITIVE},
};
use crate::{
    engine::aabb::Aabb, renderer::shader_interfaces::primitive_op_buffer::PrimitivePropsSlice,
};

// ~~ Primitive ~~

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Primitive {
    Sphere(Sphere),
    Cube(Cube),
    UberPrimitive(UberPrimitive),
}

pub const VARIANTS: &[Primitive] = &[
    Primitive::Sphere(DEFAULT_SPHERE),
    Primitive::Cube(DEFAULT_CUBE),
    Primitive::UberPrimitive(DEFAULT_UBER_PRIMITIVE),
];

pub const DEFAULT_PRIMITIVE: Primitive = Primitive::UberPrimitive(DEFAULT_UBER_PRIMITIVE);

impl Default for Primitive {
    fn default() -> Self {
        DEFAULT_PRIMITIVE
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

    fn encoded_props(&self) -> PrimitivePropsSlice {
        primitive_fn_match!(self, encoded_props)
    }

    fn aabb(&self, primitive_transform: PrimitiveTransform) -> Aabb {
        match self {
            Self::Sphere(p) => p.aabb(primitive_transform),
            Self::Cube(p) => p.aabb(primitive_transform),
            Self::UberPrimitive(p) => p.aabb(primitive_transform),
        }
    }
}

impl Primitive {
    pub fn variant_names() -> Vec<(Self, &'static str)> {
        VARIANTS
            .iter()
            .map(|primitive| (primitive.clone(), primitive.type_name()))
            .collect()
    }
}

// ~~ Encodable Primitive ~~

/// Methods required to encode and process primitive data. Mostly for GPU rendering.
pub trait EncodablePrimitive: Send + Sync {
    /// Returns the primitive type as a str
    fn type_name(&self) -> &'static str;

    /// Returns buffer compatible primitive data as a [`PrimitivePropsSlice`].
    /// `parent_origin` is the world space origin of the parent object, which should be added to
    /// the primitive center before encoding.
    ///
    /// _Note: must match the decode process in `scene_geometry.frag`_
    fn encoded_props(&self) -> PrimitivePropsSlice;

    /// Axis aligned bounding box
    fn aabb(&self, primitive_transform: PrimitiveTransform) -> Aabb;
}
