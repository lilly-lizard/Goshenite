use super::primitives::EncodablePrimitive;
use crate::shaders::shader_interfaces::{primitive_codes, PrimitiveDataSlice};
use glam::Vec3;

#[derive(Default, Debug, Clone, Copy)]
pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
}
impl Sphere {
    pub fn new(center: Vec3, radius: f32) -> Self {
        Self { center, radius }
    }
}
impl EncodablePrimitive for Sphere {
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
    /*fn decode(data: PrimitiveDataSlice) -> Option<Self> {
        if data[0] != primitive_codes::SPHERE {
            return None;
        }
        let center = Vec3::new(
            f32::from_bits(data[1]),
            f32::from_bits(data[2]),
            f32::from_bits(data[3]),
        );
        let radius = f32::from_bits(data[4]);
        Some(Self { center, radius })
    }*/
}
