use super::operation::Operation;
use crate::{
    engine::{
        aabb::Aabb,
        primitives::{null_primitive::NullPrimitive, primitive::PrimitiveCell},
    },
    helper::{
        more_errors::CollectionError,
        unique_id_gen::{UniqueId, UniqueIdGen},
    },
    renderer::shader_interfaces::primitive_op_buffer::{
        create_primitive_op_packet, nop_primitive_op_packet, PrimitiveOpBufferUnit,
        PrimitiveOpPacket,
    },
};
use egui_dnd::utils::{shift_slice, ShiftSliceError};
use glam::Vec3;
use std::{cell::RefCell, rc::Rc};

/// Use functions `borrow` and `borrow_mut` to access the `Object`.
pub type ObjectCell = RefCell<Object>;
#[inline]
pub fn new_object_ref(object: Object) -> Rc<ObjectCell> {
    Rc::new(RefCell::new(object))
}

// this is because the shaders store the primitive op index in the lower 16 bits of a u32
const MAX_PRIMITIVE_OP_COUNT: usize = u16::MAX as usize;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId(pub UniqueId);
impl ObjectId {
    pub const fn raw_id(&self) -> UniqueId {
        self.0
    }
}
impl From<UniqueId> for ObjectId {
    fn from(id: UniqueId) -> Self {
        Self(id)
    }
}
impl std::fmt::Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw_id())
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PrimitiveOpId(pub UniqueId);
impl PrimitiveOpId {
    pub const fn raw_id(&self) -> UniqueId {
        self.0
    }
}
impl From<UniqueId> for PrimitiveOpId {
    fn from(id: UniqueId) -> Self {
        Self(id)
    }
}
impl std::fmt::Display for PrimitiveOpId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw_id())
    }
}

pub struct PrimitiveOp {
    id: PrimitiveOpId,
    pub op: Operation,
    pub primitive: Rc<PrimitiveCell>,
}
impl PrimitiveOp {
    pub fn new(id: PrimitiveOpId, op: Operation, primitive: Rc<PrimitiveCell>) -> Self {
        Self { id, op, primitive }
    }

    pub fn new_default(id: PrimitiveOpId) -> Self {
        Self::new(id, Operation::NOP, NullPrimitive::new_ref())
    }

    pub fn id(&self) -> PrimitiveOpId {
        self.id
    }
}

pub struct Object {
    id: ObjectId,
    name: String,
    origin: Vec3,
    primitive_ops: Vec<PrimitiveOp>,

    primitive_op_id_gen: UniqueIdGen,
}

impl Object {
    pub fn new(
        id: ObjectId,
        name: String,
        origin: Vec3,
        base_primitive: Rc<PrimitiveCell>,
    ) -> Self {
        let mut primitive_op_id_gen = UniqueIdGen::new();
        let new_raw_id = primitive_op_id_gen
            .new_id()
            .expect("todo should probably handle this somehow...");

        Self {
            id,
            name,
            origin,
            primitive_ops: vec![PrimitiveOp::new(
                PrimitiveOpId(new_raw_id),
                Operation::Union,
                base_primitive,
            )],
            primitive_op_id_gen,
        }
    }

    pub fn remove_primitive_op(&mut self, id: PrimitiveOpId) -> Result<(), CollectionError> {
        let index = self.primitive_ops.iter().position(|p_op| p_op.id() == id);
        if let Some(index) = index {
            self.primitive_ops.remove(index);
            Ok(())
        } else {
            Err(CollectionError::InvalidId {
                raw_id: id.raw_id(),
            })
        }
        // todo recycle_id (for primitive too? on drop?)
    }

    pub fn remove_primitive_op_index(&mut self, index: usize) -> Result<(), CollectionError> {
        let op_count = self.primitive_ops.len();
        if index >= op_count {
            return Err(CollectionError::OutOfBounds {
                index,
                size: op_count,
            });
        }
        self.primitive_ops.remove(index);
        Ok(())
        // todo recycle_id
    }

    /// Returns the id of the newly created primitive op
    pub fn push_op(&mut self, operation: Operation, primitive: Rc<PrimitiveCell>) -> PrimitiveOpId {
        let new_raw_id = self
            .primitive_op_id_gen
            .new_id()
            .expect("todo should probably handle this somehow...");
        let id = PrimitiveOpId(new_raw_id);

        self.primitive_ops
            .push(PrimitiveOp::new(id, operation, primitive));
        id
    }

    pub fn shift_primitive_ops(
        &mut self,
        source_index: usize,
        target_index: usize,
    ) -> Result<(), ShiftSliceError> {
        shift_slice(source_index, target_index, &mut self.primitive_ops)
    }

    pub fn encoded_primitive_ops(&self) -> Vec<PrimitiveOpBufferUnit> {
        // avoiding this case should be the responsibility of the functions adding to `primtive_ops`
        debug_assert!(self.primitive_ops.len() <= MAX_PRIMITIVE_OP_COUNT);

        let mut encoded_primitives = Vec::<PrimitiveOpPacket>::new();
        for primitive_op in &self.primitive_ops {
            let op_code = primitive_op.op.op_code();
            let primitive_type_code = primitive_op.primitive.borrow().type_code();

            let transform = primitive_op
                .primitive
                .borrow()
                .transform()
                .encoded(self.origin);
            let props = primitive_op.primitive.borrow().encoded_props();

            let packet = create_primitive_op_packet(op_code, primitive_type_code, transform, props);
            encoded_primitives.push(packet);
        }
        if self.primitive_ops.len() == 0 {
            // having no primitive ops would probably break something so lets put a NOP here...
            let packet = nop_primitive_op_packet();
            encoded_primitives.push(packet);
        }

        let mut encoded_object = vec![
            self.id.raw_id() as PrimitiveOpBufferUnit,
            self.primitive_ops.len() as PrimitiveOpBufferUnit,
        ];
        let encoded_primitives_flattened =
            encoded_primitives.into_iter().flatten().collect::<Vec<_>>();
        encoded_object.extend_from_slice(&encoded_primitives_flattened);
        encoded_object
    }

    pub fn aabb(&self) -> Aabb {
        let mut aabb = Aabb::new_zero();
        for primitive_op in &self.primitive_ops {
            aabb.union(primitive_op.primitive.borrow().aabb());
        }
        aabb.offset(self.origin);
        aabb
    }

    pub fn set_origin(&mut self, origin: Vec3) {
        self.origin = origin;
    }
}

// Getters

impl Object {
    pub fn id(&self) -> ObjectId {
        self.id
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }

    pub fn origin(&self) -> Vec3 {
        self.origin
    }

    pub fn primitive_ops(&self) -> &Vec<PrimitiveOp> {
        &self.primitive_ops
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
}
