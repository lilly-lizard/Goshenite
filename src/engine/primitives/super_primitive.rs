use super::{
    primitive::{primitive_names, EncodablePrimitive},
    primitive_transform::PrimitiveTransform,
};
use crate::{
    engine::aabb::Aabb, renderer::shader_interfaces::primitive_op_buffer::PrimitivePropsSlice,
};
use glam::{Quat, Vec2, Vec3, Vec4};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SuperPrimitive {
    pub transform: PrimitiveTransform,
    pub s: Vec4,
    pub r: Vec2,
}

impl SuperPrimitive {
    pub const fn new(center: Vec3, rotation: Quat, s: Vec4, r: Vec2) -> Self {
        let transform = PrimitiveTransform { center, rotation };
        Self { transform, s, r }
    }
}

impl Default for SuperPrimitive {
    fn default() -> Self {
        Self {
            transform: PrimitiveTransform::default(),
            s: Vec4::ZERO,
            r: Vec2::ZERO,
        }
    }
}

impl EncodablePrimitive for SuperPrimitive {
    fn type_name(&self) -> &'static str {
        primitive_names::SUPER_PRIMITIVE
    }

    fn encoded_props(&self) -> PrimitivePropsSlice {
        [
            self.s.x.to_bits(),
            self.s.y.to_bits(),
            self.s.z.to_bits(),
            self.s.w.to_bits(),
            self.r.x.to_bits(),
            self.r.y.to_bits(),
        ]
    }

    fn transform(&self) -> &PrimitiveTransform {
        &self.transform
    }

    fn aabb(&self) -> Aabb {
        // todo calculate only when props/transform changed!
        //todo "dimensions need to be adjusted for rotation!
        Aabb::new(self.transform.center, Vec3::new(2.0, 2.0, 2.0))
    }
}
