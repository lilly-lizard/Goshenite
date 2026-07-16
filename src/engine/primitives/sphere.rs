use super::primitive::EncodablePrimitive;
use crate::engine::{
    aabb::Aabb,
    config_engine::{primitive_names, DEFAULT_RADIUS},
    primitives::transform::Transform,
};
use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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

    fn uber_s(&self) -> [f32; 4] {
        let width = 0_f32;
        let depth = 0_f32;
        let height = self.radius;
        let thickness = self.radius;
        [width, depth, height, thickness]
    }

    fn uber_r(&self) -> [f32; 2] {
        [0., self.radius]
    }

    fn aabb(&self, _primitive_transform: Transform) -> Aabb {
        // todo calculate only when props/transform changed? will need to make members private...
        Aabb::new(Vec3::splat(2. * self.radius))
    }
}
