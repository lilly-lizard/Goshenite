use glam::{Quat, Vec3};

use super::{
    cube::Cube,
    null_primitive::NullPrimitive,
    primitive::{default_dimensions, default_radius, Primitive},
    sphere::Sphere,
};

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
