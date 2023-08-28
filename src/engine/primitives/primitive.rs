use super::{
    cube::Cube, null_primitive::NullPrimitive, primitive_transform::PrimitiveTransform,
    sphere::Sphere,
};
use crate::{
    engine::aabb::Aabb,
    renderer::shader_interfaces::primitive_op_buffer::{
        PrimitiveOpBufferUnit, PrimitivePropsSlice,
    },
};
use glam::Vec3;

pub const DEFAULT_RADIUS: f32 = 0.5;
pub const DEFAULT_DIMENSIONS: Vec3 = Vec3::ONE;

// PRIMITIVE

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Primitive {
    Null(NullPrimitive),
    Sphere(Sphere),
    Cube(Cube),
}

impl Default for Primitive {
    fn default() -> Self {
        Self::Null(NullPrimitive::new())
    }
}

macro_rules! primitive_fn_match {
    ($self:ident, $primitive_fn:ident) => {
        match $self {
            Self::Null(p) => p.$primitive_fn(),
            Self::Sphere(p) => p.$primitive_fn(),
            Self::Cube(p) => p.$primitive_fn(),
        }
    };
}

impl EncodablePrimitive for Primitive {
    fn type_code(&self) -> PrimitiveOpBufferUnit {
        primitive_fn_match!(self, type_code)
    }
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

// ENCODABLE PRIMITIVE

/// Methods required to encode and process primitive data. Mostly for GPU rendering.
pub trait EncodablePrimitive: Send + Sync {
    /// Returns the primitive type code. See [`primitive_type_codes`].
    fn type_code(&self) -> PrimitiveOpBufferUnit;

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

// CONSTANTS

pub mod primitive_names {
    use super::Primitive;
    use crate::engine::primitives::{cube::Cube, null_primitive::NullPrimitive, sphere::Sphere};

    pub const NULL: &'static str = "Null-Primitive";
    pub const SPHERE: &'static str = "Sphere";
    pub const CUBE: &'static str = "Cube";

    pub const NAME_LIST: [&'static str; 3] = [NULL, SPHERE, CUBE];

    pub fn default_primitive_from_type_name(type_name: &'static str) -> Primitive {
        match type_name {
            SPHERE => Primitive::Sphere(Sphere::default()),
            CUBE => Primitive::Cube(Cube::default()),
            _ => Primitive::Null(NullPrimitive::default()),
        }
    }
}
