use super::operation::Operation;
use super::primitives::{none::None, primitive::Primitive};
use std::rc::Rc;

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
    primitive_ops: Vec<PrimitiveOp>,
}
impl Object {
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
}
