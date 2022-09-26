use crate::shaders::shader_interfaces::{primitve_codes, PrimitiveDataUnit, PRIMITIVE_LEN};
use glam::Vec3;

/// bruh
type PrimitiveDataSlice = [PrimitiveDataUnit; PRIMITIVE_LEN];

#[derive(Default, Debug, Clone)]
pub struct Primitives {
    /// Encoded primitive data.
    data: Vec<PrimitiveDataUnit>,
}
// Public functions
impl Primitives {
    pub fn encoded_data(&self) -> &Vec<PrimitiveDataUnit> {
        &self.data
    }

    pub fn add_sphere(&mut self, radius: f32, center: Vec3) {
        self.data
            .extend_from_slice(&Self::encode_sphere(radius, center));
    }
}
// Private functions
impl Primitives {
    fn encode_sphere(radius: f32, center: Vec3) -> PrimitiveDataSlice {
        [
            primitve_codes::SPHERE,
            radius.to_bits(),
            center.x.to_bits(),
            center.y.to_bits(),
            center.z.to_bits(),
            // padding
            primitve_codes::NULL,
            primitve_codes::NULL,
            primitve_codes::NULL,
        ]
    }
}
