use super::primitive::Primitive;
use crate::shaders::object_buffer::{primitive_codes, PrimitiveDataSlice, PRIMITIVE_UNIT_LEN};
use glam::Vec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct None {}
impl Primitive for None {
    fn encode(&self) -> PrimitiveDataSlice {
        [primitive_codes::NULL; PRIMITIVE_UNIT_LEN]
    }

    fn center(&self) -> Vec3 {
        Vec3::ZERO
    }

    fn type_name(&self) -> &'static str {
        "None"
    }
}
impl Default for None {
    fn default() -> Self {
        Self {}
    }
}
