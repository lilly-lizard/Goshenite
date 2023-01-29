use super::operation::Operation;
use crate::{
    engine::primitives::{
        null_primitive::NullPrimitive,
        primitive::{new_primitive_ref, PrimitiveRef},
    },
    helper::unique_id_gen::{UniqueId, UniqueIdGen},
    renderer::shader_interfaces::object_buffer::ObjectDataUnit,
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

pub type ObjectId = UniqueId;
pub type PrimitiveOpId = UniqueId;

pub struct PrimitiveOp {
    id: PrimitiveOpId,
    pub op: Operation,
    pub prim: Rc<PrimitiveRef>,
}
impl PrimitiveOp {
    pub fn new(id: PrimitiveOpId, op: Operation, primitive: Rc<PrimitiveRef>) -> Self {
        Self {
            id,
            op,
            prim: primitive,
        }
    }
    pub fn new_default(id: PrimitiveOpId) -> Self {
        Self::new(id, Operation::NOP, new_primitive_ref(NullPrimitive {}))
    }
    pub fn id(&self) -> PrimitiveOpId {
        self.id
    }
}

pub struct Object {
    id: ObjectId,
    primitive_op_id_gen: UniqueIdGen,
    pub name: String,
    pub origin: Vec3,
    pub primitive_ops: Vec<PrimitiveOp>,
}
impl Object {
    pub fn new(id: ObjectId, name: String, origin: Vec3, base_primitive: Rc<PrimitiveRef>) -> Self {
        let mut primitive_op_id_gen = UniqueIdGen::new();
        Self {
            id,
            name,
            origin,
            primitive_ops: vec![PrimitiveOp::new(
                primitive_op_id_gen.new_id(),
                Operation::Union,
                base_primitive,
            )],
            primitive_op_id_gen,
        }
    }

    pub fn id(&self) -> ObjectId {
        self.id
    }

    /// If found, returns a tuple with the vec index and a ref to the primitive op
    pub fn get_primitive_op(&self, id: PrimitiveOpId) -> Option<(usize, &PrimitiveOp)> {
        self.primitive_ops
            .iter()
            .enumerate()
            .find_map(|(index, prim_op)| {
                if prim_op.id() == id {
                    Some((index, prim_op))
                } else {
                    None
                }
            })
    }

    /// If found, returns a tuple with the vec index and a ref to the primitive op
    pub fn get_primitive_op_mut(&mut self, id: PrimitiveOpId) -> Option<(usize, &mut PrimitiveOp)> {
        self.primitive_ops
            .iter_mut()
            .enumerate()
            .find_map(|(index, prim_op)| {
                if prim_op.id() == id {
                    Some((index, prim_op))
                } else {
                    None
                }
            })
    }

    pub fn push_op(&mut self, operation: Operation, primitive: Rc<PrimitiveRef>) {
        self.primitive_ops.push(PrimitiveOp::new(
            self.primitive_op_id_gen.new_id(),
            operation,
            primitive,
        ));
    }

    pub fn encoded_data(&self) -> Vec<ObjectDataUnit> {
        // avoiding this case should be the responsibility of the functions adding to `primtive_ops`
        debug_assert!(self.primitive_ops.len() <= MAX_PRIMITIVE_OP_COUNT);
        let mut encoded = vec![
            self.id as ObjectDataUnit,
            self.primitive_ops.len() as ObjectDataUnit,
        ];
        for primitive_op in &self.primitive_ops {
            encoded.push(primitive_op.op.op_code());
            encoded.extend_from_slice(&primitive_op.prim.borrow().encode(self.origin));
        }
        encoded
    }
}
