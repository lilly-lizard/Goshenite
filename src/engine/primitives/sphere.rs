use super::{
    primitive::{primitive_names, EncodablePrimitive, DEFAULT_RADIUS},
    primitive_transform::{default_primitive_transform, PrimitiveTransform},
};
use crate::{
    engine::aabb::Aabb, renderer::shader_interfaces::primitive_op_buffer::PrimitivePropsSlice,
};
use glam::{Quat, Vec2, Vec3};

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

#[inline]
pub const fn default_sphere() -> Sphere {
    Sphere {
        transform: default_primitive_transform(),
        radius: DEFAULT_RADIUS,
    }
}

impl Default for Sphere {
    fn default() -> Self {
        default_sphere()
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

    fn transform(&self) -> &PrimitiveTransform {
        &self.transform
    }

    fn aabb(&self) -> Aabb {
        // todo calculate only when props/transform changed? will need to make members private...
        Aabb::new(self.transform.center, Vec3::splat(2. * self.radius))
    }
}
