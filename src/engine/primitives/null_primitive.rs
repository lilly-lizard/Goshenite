use super::{
    primitive::{Primitive, PrimitiveId},
    primitive_transform::PrimitiveTransform,
};
use crate::{
    engine::aabb::Aabb,
    renderer::shader_interfaces::primitive_op_buffer::{
        primitive_type_codes, PrimitiveOpBufferUnit, PrimitivePropsSlice, PRIMITIVE_PROPS_LEN,
    },
};

const NULL_TRANSFORM: PrimitiveTransform = PrimitiveTransform::new_default();

#[derive(Debug, Clone, PartialEq)]
pub struct NullPrimitive {}

impl NullPrimitive {
    #[inline]
    pub fn new() -> Self {
        Self {}
    }
}

impl Primitive for NullPrimitive {
    fn id(&self) -> PrimitiveId {
        usize::MAX.into()
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

impl Default for NullPrimitive {
    fn default() -> Self {
        Self {}
    }
}
