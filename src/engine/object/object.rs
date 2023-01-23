use super::operation::Operation;
use crate::{
    engine::primitives::{
        none::None,
        primitive::{new_primitive_ref, Primitive, PrimitiveRef},
    },
    renderer::shaders::object_buffer::ObjectDataUnit,
};
use glam::Vec3;
use std::{cell::RefCell, rc::Rc};

/// Use functions `borrow` and `borrow_mut` to access the `Object`.
pub type ObjectRef = RefCell<Object>;
#[inline]
pub fn new_object_ref(object: Object) -> Rc<ObjectRef> {
    Rc::new(RefCell::new(object))
}

// this is because the shaders store the primitive op index in the lower 16 bits of a u32
const MAX_PRIMITIVE_OP_COUNT: usize = u16::MAX as usize;

pub struct PrimitiveOp {
    pub op: Operation,
    pub prim: Rc<PrimitiveRef>,
}
impl Default for PrimitiveOp {
    fn default() -> Self {
        Self {
            op: Operation::None,
            prim: new_primitive_ref(None {}),
        }
    }
}

pub struct Object {
    id: usize,
    pub name: String,
    pub origin: Vec3,
    pub primitive_ops: Vec<PrimitiveOp>,
}
impl Object {
    pub fn new(id: usize, name: String, origin: Vec3, base_primitive: Rc<PrimitiveRef>) -> Self {
        Self {
            id,
            name,
            origin,
            primitive_ops: vec![PrimitiveOp {
                op: Operation::Union,
                prim: base_primitive,
            }],
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn push_op(&mut self, operation: Operation, primitive: Rc<PrimitiveRef>) {
        self.primitive_ops.push(PrimitiveOp {
            op: operation,
            prim: primitive,
        });
    }

    pub fn encoded_data(&self) -> Vec<ObjectDataUnit> {
        // avoiding this case should be the responsibility of the functions adding to `primtive_ops`
        debug_assert!(self.primitive_ops.len() <= MAX_PRIMITIVE_OP_COUNT);
        let mut encoded = vec![self.primitive_ops.len() as ObjectDataUnit];
        for primitive_op in &self.primitive_ops {
            encoded.push(primitive_op.op.op_code());
            encoded.extend_from_slice(&primitive_op.prim.borrow().encode(self.origin));
        }
        encoded
    }
}
