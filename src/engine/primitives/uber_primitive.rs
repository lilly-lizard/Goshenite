use super::{
    primitive::EncodablePrimitive,
    primitive_transform::{PrimitiveTransform, DEFAULT_PRIMITIVE_TRANSFORM},
};
use crate::{
    engine::{aabb::Aabb, config_engine::primitive_names},
    renderer::shader_interfaces::primitive_op_buffer::PrimitivePropsSlice,
};
use glam::{Quat, Vec2, Vec3, Vec4};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UberPrimitive {
    pub transform: PrimitiveTransform,
    /// width, depth, height, thickness
    pub dimensions: Vec4,
    pub corner_radius: Vec2,
}

impl UberPrimitive {
    pub const fn new(center: Vec3, rotation: Quat, dimensions: Vec4, corner_radius: Vec2) -> Self {
        let transform = PrimitiveTransform::new(center, rotation);
        Self {
            transform,
            dimensions,
            corner_radius,
        }
    }
}

pub const DEFAULT_UBER_PRIMITIVE: UberPrimitive = UberPrimitive {
    transform: DEFAULT_PRIMITIVE_TRANSFORM,
    dimensions: Vec4::ZERO,
    corner_radius: Vec2::ZERO,
};

impl Default for UberPrimitive {
    fn default() -> Self {
        DEFAULT_UBER_PRIMITIVE
    }
}

impl EncodablePrimitive for UberPrimitive {
    fn type_name(&self) -> &'static str {
        primitive_names::UBER_PRIMITIVE
    }

    fn encoded_props(&self) -> PrimitivePropsSlice {
        [
            self.dimensions.x.to_bits(),
            self.dimensions.y.to_bits(),
            self.dimensions.z.to_bits(),
            self.dimensions.w.to_bits(),
            self.corner_radius.x.to_bits(),
            self.corner_radius.y.to_bits(),
        ]
    }

    fn transform(&self) -> &PrimitiveTransform {
        &self.transform
    }

    fn aabb(&self) -> Aabb {
        // todo calculate only when props/transform changed?
        //todo "dimensions need to be adjusted for rotation!
        let max_dimensions = Vec3::new(5., 5., 5.);
        Aabb::new(self.transform.center, max_dimensions)
    }
}
