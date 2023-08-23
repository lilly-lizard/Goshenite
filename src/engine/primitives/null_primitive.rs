use super::{
    primitive::{primitive_names, EncodablePrimitive},
    primitive_transform::PrimitiveTransform,
};
use crate::{
    engine::aabb::Aabb,
    renderer::shader_interfaces::primitive_op_buffer::{
        primitive_type_codes, PrimitiveOpBufferUnit, PrimitivePropsSlice, PRIMITIVE_PROPS_LEN,
    },
};

const NULL_TRANSFORM: PrimitiveTransform = PrimitiveTransform::new_default();

#[derive(Default, Debug, Clone, PartialEq)]
pub struct NullPrimitive {}

impl NullPrimitive {
    #[inline]
    pub fn new() -> Self {
        Self {}
    }
}

impl EncodablePrimitive for NullPrimitive {
    fn type_code(&self) -> PrimitiveOpBufferUnit {
        primitive_type_codes::NULL
    }

    fn type_name(&self) -> &'static str {
        primitive_names::NULL
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
