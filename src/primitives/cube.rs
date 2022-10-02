use super::primitives::EncodablePrimitive;
use crate::shaders::shader_interfaces::{primitive_codes, PrimitiveDataSlice};
use glam::Vec3;

#[derive(Default, Debug, Clone, Copy)]
pub struct Cube {
    pub center: Vec3,
    pub dimensions: Vec3,
}
impl Cube {
    pub fn new(center: Vec3, dimensions: Vec3) -> Self {
        Self { center, dimensions }
    }
}
impl EncodablePrimitive for Cube {
    fn encode(&self) -> PrimitiveDataSlice {
        [
            primitive_codes::CUBE,
            self.center.x.to_bits(),
            self.center.y.to_bits(),
            self.center.z.to_bits(),
            self.dimensions.x.to_bits(),
            self.dimensions.y.to_bits(),
            self.dimensions.z.to_bits(),
            // padding
            primitive_codes::NULL,
        ]
    }
    /*fn decode(data: PrimitiveDataSlice) -> Option<Self> {
        if data[0] != primitive_codes::CUBE {
            return None;
        }
        let center = Vec3::new(
            f32::from_bits(data[1]),
            f32::from_bits(data[2]),
            f32::from_bits(data[3]),
        );
        let dimensions = Vec3::new(
            f32::from_bits(data[4]),
            f32::from_bits(data[5]),
            f32::from_bits(data[6]),
        );
        Some(Self { center, dimensions })
    }*/

    fn center(&self) -> Vec3 {
        self.center
    }
}
