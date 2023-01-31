use super::primitive::Primitive;
use crate::{
    engine::aabb::Aabb,
    renderer::shader_interfaces::object_buffer::{
        primitive_codes, PrimitiveDataSlice, PRIMITIVE_UNIT_LEN,
    },
};
use glam::Vec3;
use std::{cell::RefCell, rc::Rc};

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

    fn aabb(&self) -> Aabb {
        Aabb::new_zero()
    }
}

impl NullPrimitive {
    pub fn new_ref() -> Rc<RefCell<NullPrimitive>> {
        Rc::new(RefCell::new(NullPrimitive {}))
    }
}

impl Default for NullPrimitive {
    fn default() -> Self {
        Self {}
    }
}
