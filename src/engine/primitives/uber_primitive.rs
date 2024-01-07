use super::{primitive::EncodablePrimitive, primitive_transform::PrimitiveTransform};
use crate::{
    engine::{aabb::Aabb, config_engine::primitive_names},
    renderer::shader_interfaces::primitive_op_buffer::PrimitivePropsSlice,
};
use glam::{Vec2, Vec3, Vec4};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct UberPrimitive {
    /// width, depth, height, thickness
    pub dimensions: Vec4,
    pub corner_radius: Vec2,
}

impl UberPrimitive {
    pub const fn new(dimensions: Vec4, corner_radius: Vec2) -> Self {
        Self {
            dimensions,
            corner_radius,
        }
    }

    pub const DEFAULT: UberPrimitive = UberPrimitive {
        dimensions: Vec4::ZERO,
        corner_radius: Vec2::ZERO,
    };
}

impl Default for UberPrimitive {
    fn default() -> Self {
        Self::DEFAULT
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

    fn aabb(&self, primitive_transform: PrimitiveTransform) -> Aabb {
        // todo calculate only when props/transform changed?
        //todo "dimensions need to be adjusted for rotation!
        let max_dimensions = Vec3::new(5., 5., 5.);
        Aabb::new(primitive_transform.center, max_dimensions)
    }
}
