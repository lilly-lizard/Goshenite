use super::{
    primitive::{primitive_names, EncodablePrimitive, DEFAULT_RADIUS},
    primitive_transform::PrimitiveTransform,
};
use crate::{
    engine::aabb::Aabb, renderer::shader_interfaces::primitive_op_buffer::PrimitivePropsSlice,
};
use glam::{Quat, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
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
            radius: DEFAULT_RADIUS,
        }
    }
}

impl EncodablePrimitive for Sphere {
    fn type_name(&self) -> &'static str {
        primitive_names::SPHERE
    }

    fn encoded_props(&self) -> PrimitivePropsSlice {
        [
            self.radius.to_bits(),
            0_f32.to_bits(),
            0_f32.to_bits(),
            0_f32.to_bits(),
            0_f32.to_bits(),
            0_f32.to_bits(),
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
