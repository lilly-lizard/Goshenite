use super::{primitive::EncodablePrimitive, transform::Transform};
use crate::engine::{aabb::Aabb, config_engine::primitive_names};
use glam::{Vec2, Vec3, Vec4};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct UberPrimitive {
    /// width, depth, height, thickness
    pub dimensions: Vec4,
    pub corner_radius: Vec2,
}

impl UberPrimitive {
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

    fn uber_s(&self) -> [f32; 4] {
        [
            self.dimensions.x,
            self.dimensions.y,
            self.dimensions.z,
            self.dimensions.w,
        ]
    }

    fn uber_r(&self) -> [f32; 2] {
        [self.corner_radius.x, self.corner_radius.y]
    }

    fn aabb(&self, _primitive_transform: Transform) -> Aabb {
        // todo calculate only when props/transform changed?
        // todo "dimensions need to be adjusted for rotation!
        let max_dimensions = Vec3::new(5., 5., 5.);
        Aabb::new(max_dimensions)
    }
}
