use super::primitive::PrimitiveTrait;
use crate::shaders::primitive_buffer::{primitive_codes, PrimitiveDataSlice};
use glam::Vec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cube {
    pub center: Vec3,
    pub dimensions: Vec3,
}
impl Cube {
    pub const fn new(center: Vec3, dimensions: Vec3) -> Self {
        Self { center, dimensions }
    }
}
impl PrimitiveTrait for Cube {
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

    fn center(&self) -> Vec3 {
        self.center
    }

    fn type_name(&self) -> &'static str {
        "Cube"
    }
}
impl Default for Cube {
    fn default() -> Self {
        Self {
            center: Vec3::ZERO,
            dimensions: Vec3::ONE,
        }
    }
}
