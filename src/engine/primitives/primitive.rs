use super::{
    cube::Cube, primitive_transform::PrimitiveTransform, sphere::Sphere,
    uber_primitive::UberPrimitive,
};
use crate::{
    engine::aabb::Aabb, helper::from_enum_macro::impl_from_for_enum_variant,
    renderer::shader_interfaces::primitive_op_buffer::PrimitivePropsSlice,
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

impl_from_for_enum_variant!(Primitive, Cube);
impl_from_for_enum_variant!(Primitive, Sphere);
impl_from_for_enum_variant!(Primitive, UberPrimitive);

// ~~ Encodable Primitive ~~

/// Methods required to encode and process primitive data. Mostly for GPU rendering.
pub trait EncodablePrimitive: Send + Sync + Serialize {
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
