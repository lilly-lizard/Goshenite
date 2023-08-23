use glam::{Quat, Vec3};

use super::{
    cube::Cube,
    null_primitive::NullPrimitive,
    primitive::{default_dimensions, default_radius, Primitive},
    sphere::Sphere,
};

/// Implimentations of [`Primtive`] supported by [`PrimitiveReferences`] return one of these values
/// when calling [`Primtive::type_name`]
pub mod primitive_names {
    pub const NULL: &'static str = "Null Primitive";
    pub const SPHERE: &'static str = "Sphere";
    pub const CUBE: &'static str = "Cube";
}

static VARIANTS: &[PrimitiveRefType] = &[
    PrimitiveRefType::Null,
    PrimitiveRefType::Sphere,
    PrimitiveRefType::Cube,
    // PrimitiveRefType::Unknown -> shouldn't be shown in lists
];

/// Possible primitive variations supported by [`PrimitiveReferences`]
#[derive(PartialEq, Clone, Copy)]
pub enum PrimitiveRefType {
    Null,
    Sphere,
    Cube,
    Unknown,
}

impl PrimitiveRefType {
    pub fn variant_names() -> Vec<(Self, &'static str)> {
        VARIANTS
            .iter()
            .map(|&p| (p, p.into()))
            .collect::<Vec<(Self, &'static str)>>()
    }
}

impl From<PrimitiveRefType> for &str {
    fn from(value: PrimitiveRefType) -> Self {
        match value {
            PrimitiveRefType::Null => primitive_names::NULL,
            PrimitiveRefType::Sphere => primitive_names::SPHERE,
            PrimitiveRefType::Cube => primitive_names::CUBE,
            PrimitiveRefType::Unknown => "Unknown",
        }
    }
}
impl From<&str> for PrimitiveRefType {
    fn from(name: &str) -> Self {
        match name {
            primitive_names::NULL => PrimitiveRefType::Null,
            primitive_names::SPHERE => PrimitiveRefType::Sphere,
            primitive_names::CUBE => PrimitiveRefType::Cube,
            _ => PrimitiveRefType::Unknown,
        }
    }
}

impl Default for PrimitiveRefType {
    fn default() -> Self {
        Self::Null
    }
}

pub fn create_default_primitive(primitive_type: PrimitiveRefType) -> Box<dyn Primitive> {
    match primitive_type {
        PrimitiveRefType::Null | PrimitiveRefType::Unknown => Box::new(NullPrimitive::new()),
        PrimitiveRefType::Sphere => {
            Box::new(Sphere::new(Vec3::ZERO, Quat::IDENTITY, default_radius()))
        }
        PrimitiveRefType::Cube => {
            Box::new(Cube::new(Vec3::ZERO, Quat::IDENTITY, default_dimensions()))
        }
    }
}
