use super::{
    primitive::{default_radius, primitive_names, EncodablePrimitive},
    primitive_transform::PrimitiveTransform,
};
use crate::{
    engine::aabb::Aabb,
    renderer::shader_interfaces::primitive_op_buffer::{
        primitive_type_codes, PrimitiveOpBufferUnit, PrimitivePropsSlice,
    },
};
use glam::{Quat, Vec3};

#[derive(Debug, Clone, PartialEq)]
pub struct Sphere {
    pub transform: PrimitiveTransform,
    pub radius: f32,
}

impl Sphere {
    pub const fn new(center: Vec3, rotation: Quat, radius: f32) -> Self {
        let transform = PrimitiveTransform { center, rotation };
        Self { transform, radius }
    }
}

impl Default for Sphere {
    fn default() -> Self {
        Self {
            transform: PrimitiveTransform::default(),
            radius: default_radius(),
        }
    }
}

impl EncodablePrimitive for Sphere {
    fn type_code(&self) -> PrimitiveOpBufferUnit {
        primitive_type_codes::SPHERE
    }

    fn type_name(&self) -> &'static str {
        primitive_names::SPHERE
    }

    fn encoded_props(&self) -> PrimitivePropsSlice {
        [
            self.radius.to_bits(),
            // padding
            0,
            0,
            0,
            0,
            0,
        ]
    }

    fn transform(&self) -> &PrimitiveTransform {
        &self.transform
    }

    fn aabb(&self) -> Aabb {
        // todo calculate only when props/transform changed? will need to make members private...
        Aabb::new(self.transform.center, Vec3::splat(2. * self.radius))
    }
}
