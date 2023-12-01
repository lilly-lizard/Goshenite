use super::{
    cube::{default_cube, Cube},
    primitive_transform::PrimitiveTransform,
    sphere::{default_sphere, Sphere},
    uber_primitive::{default_uber_primitive, UberPrimitive},
};
use crate::{
    engine::aabb::Aabb, renderer::shader_interfaces::primitive_op_buffer::PrimitivePropsSlice,
};
use glam::Vec3;

// ~~ Constants ~~

pub const DEFAULT_RADIUS: f32 = 0.5;
pub const DEFAULT_DIMENSIONS: Vec3 = Vec3::ONE;

pub mod primitive_names {
    pub const SPHERE: &'static str = "Sphere";
    pub const CUBE: &'static str = "Cube";
    pub const UBER_PRIMITIVE: &'static str = "Uber Primitive";
}

// ~~ Primitive ~~

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Primitive {
    Sphere(Sphere),
    Cube(Cube),
    UberPrimitive(UberPrimitive),
}

static VARIANTS: &[Primitive] = &[
    Primitive::Sphere(default_sphere()),
    Primitive::Cube(default_cube()),
    Primitive::UberPrimitive(default_uber_primitive()),
];

impl Default for Primitive {
    fn default() -> Self {
        Self::UberPrimitive(Default::default())
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
    fn transform(&self) -> &PrimitiveTransform {
        primitive_fn_match!(self, transform)
    }
    fn aabb(&self) -> Aabb {
        primitive_fn_match!(self, aabb)
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

    /// Returns a reference to the primitive tranform of this instance
    fn transform(&self) -> &PrimitiveTransform;

    /// Axis aligned bounding box
    fn aabb(&self) -> Aabb;
}
