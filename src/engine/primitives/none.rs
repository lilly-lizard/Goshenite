use super::primitive::Primitive;
use crate::renderer::shaders::object_buffer::{
    primitive_codes, PrimitiveDataSlice, PRIMITIVE_UNIT_LEN,
};
use glam::Vec3;

#[derive(Debug, Clone, PartialEq)]
pub struct None {}
impl Primitive for None {
    fn id(&self) -> usize {
        usize::MAX
    }
    fn encode(&self, parent_origin: Vec3) -> PrimitiveDataSlice {
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
