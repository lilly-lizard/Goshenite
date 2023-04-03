use super::{primitive::Primitive, primitive_transform::PrimitiveTransform};
use crate::{
    engine::aabb::Aabb,
    renderer::shader_interfaces::primitive_op_buffer::{
        primitive_type_codes, PrimitiveOpBufferUnit, PrimitivePropsSlice, PRIMITIVE_PROPS_LEN,
    },
};
use std::{cell::RefCell, rc::Rc};

const NULL_TRANSFORM: PrimitiveTransform = PrimitiveTransform::new_default();

#[derive(Debug, Clone, PartialEq)]
pub struct NullPrimitive {}

impl Primitive for NullPrimitive {
    fn id(&self) -> usize {
        usize::MAX
    }

    fn type_code(&self) -> PrimitiveOpBufferUnit {
        primitive_type_codes::NULL
    }

    fn type_name(&self) -> &'static str {
        "Null-Primitive"
    }

    fn encoded_props(&self) -> PrimitivePropsSlice {
        [0; PRIMITIVE_PROPS_LEN]
    }

    fn transform(&self) -> &PrimitiveTransform {
        &NULL_TRANSFORM
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
