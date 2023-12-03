use super::{primitive::EncodablePrimitive, primitive_transform::PrimitiveTransform};
use crate::{
    engine::{
        aabb::Aabb,
        config_engine::{primitive_names, DEFAULT_RADIUS},
    },
    renderer::shader_interfaces::primitive_op_buffer::PrimitivePropsSlice,
};
use glam::{Vec2, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sphere {
    pub radius: f32,
}

impl Sphere {
    pub const fn new(radius: f32) -> Self {
        Self { radius }
    }

    pub const DEFAULT: Self = Self {
        radius: DEFAULT_RADIUS,
    };
}

impl Default for Sphere {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl EncodablePrimitive for Sphere {
    fn type_name(&self) -> &'static str {
        primitive_names::SPHERE
    }

    fn encoded_props(&self) -> PrimitivePropsSlice {
        let width = 0_f32;
        let depth = 0_f32;
        let height = self.radius;
        let thickness = self.radius;
        let corner_radius = Vec2::new(0_f32, self.radius);
        [
            width.to_bits(),
            depth.to_bits(),
            height.to_bits(),
            thickness.to_bits(),
            corner_radius.x.to_bits(),
            corner_radius.y.to_bits(),
        ]
    }

    fn aabb(&self, primitive_transform: PrimitiveTransform) -> Aabb {
        // todo calculate only when props/transform changed? will need to make members private...
        Aabb::new(primitive_transform.center, Vec3::splat(2. * self.radius))
    }
}
