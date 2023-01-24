use super::primitive::Primitive;
use crate::renderer::shader_interfaces::object_buffer::{
    primitive_codes, PrimitiveDataSlice, PRIMITIVE_UNIT_LEN,
};
use glam::Vec3;

#[derive(Debug, Clone, PartialEq)]
pub struct NullPrimitive {}
impl Primitive for NullPrimitive {
    fn id(&self) -> usize {
        usize::MAX
    }

    fn encode(&self, _parent_origin: Vec3) -> PrimitiveDataSlice {
        [primitive_codes::NULL; PRIMITIVE_UNIT_LEN]
    }

    fn center(&self) -> Vec3 {
        Vec3::ZERO
    }

    fn type_name(&self) -> &'static str {
        "Null-Primitive"
    }
}
impl Default for NullPrimitive {
    fn default() -> Self {
        Self {}
    }
}
