use super::primitive::PrimitiveTrait;
use crate::shaders::shader_interfaces::{primitive_codes, PrimitiveDataSlice};
use glam::Vec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
}
impl Sphere {
    pub fn new(center: Vec3, radius: f32) -> Self {
        Self { center, radius }
    }
}
impl PrimitiveTrait for Sphere {
    fn encode(&self) -> PrimitiveDataSlice {
        [
            primitive_codes::SPHERE,
            self.center.x.to_bits(),
            self.center.y.to_bits(),
            self.center.z.to_bits(),
            self.radius.to_bits(),
            // padding
            primitive_codes::NULL,
            primitive_codes::NULL,
            primitive_codes::NULL,
        ]
    }
    fn center(&self) -> Vec3 {
        self.center
    }
}
impl Default for Sphere {
    fn default() -> Self {
        Self {
            center: Vec3::ZERO,
            radius: 1.0,
        }
    }
}
