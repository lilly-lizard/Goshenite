use glam::Vec3;

use super::operation::Operation;
use crate::{
    engine::primitives::{none::None, primitive::Primitive},
    renderer::shaders::object_buffer::ObjectDataUnit,
};
use std::rc::Rc;

// this is because the shaders store the primitive op index in the lower 16 bits of a u32
const MAX_PRIMITIVE_OP_COUNT: usize = u16::MAX as usize;

pub struct PrimitiveOp {
    pub op: Operation,
    pub pr: Rc<dyn Primitive>,
}
impl Default for PrimitiveOp {
    fn default() -> Self {
        Self {
            op: Operation::None,
            pr: Rc::new(None {}),
        }
    }
}

pub struct Object {
    origin: Vec3,
    primitive_ops: Vec<PrimitiveOp>,
}
impl Object {
    pub fn new(origin: Vec3, base_primitive: Rc<dyn Primitive>) -> Self {
        Self {
            origin,
            primitive_ops: vec![PrimitiveOp {
                op: Operation::Union,
                pr: base_primitive,
            }],
        }
    }

    pub fn origin(&self) -> Vec3 {
        self.origin
    }
    pub fn set_origin(&mut self, origin: Vec3) {
        self.origin = origin;
    }

    pub fn primitive_ops(&self) -> &Vec<PrimitiveOp> {
        &self.primitive_ops
    }
    pub fn primitive_ops_mut(&mut self) -> &mut Vec<PrimitiveOp> {
        &mut self.primitive_ops
    }

    pub fn append(&mut self, operation: Operation, primitive: Rc<dyn Primitive>) {
        self.primitive_ops.push(PrimitiveOp {
            op: operation,
            pr: primitive,
        });
    }

    pub fn encoded_data(&self) -> Vec<ObjectDataUnit> {
        // avoiding this case should be the responsibility of the functions adding to `primtive_ops`
        debug_assert!(self.primitive_ops.len() <= MAX_PRIMITIVE_OP_COUNT);
        let mut encoded = vec![self.primitive_ops.len() as ObjectDataUnit];
        for primitive_op in &self.primitive_ops {
            encoded.push(primitive_op.op.op_code());
            encoded.extend_from_slice(&primitive_op.pr.encode(self.origin));
        }
        encoded
    }
}
