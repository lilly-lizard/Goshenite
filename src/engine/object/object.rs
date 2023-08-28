use super::{
    operation::Operation,
    primitive_op::{PrimitiveOp, PrimitiveOpDuplicate, PrimitiveOpId},
};
use crate::{
    engine::{
        aabb::Aabb,
        primitives::primitive::{EncodablePrimitive, Primitive},
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

// this is because the shaders store the primitive op index in the lower 16 bits of a u32
const MAX_PRIMITIVE_OP_COUNT: usize = u16::MAX as usize;

// OBJECT ID

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

// OBJECT

pub struct Object {
    id: ObjectId,
    pub name: String,
    pub origin: Vec3,
    pub primitive_ops: Vec<PrimitiveOp>,

    primitive_op_id_gen: UniqueIdGen,
}

impl Object {
    pub fn new(id: ObjectId, name: String, origin: Vec3) -> Self {
        Self {
            id,
            name,
            origin,
            primitive_ops: vec![],
            primitive_op_id_gen: UniqueIdGen::new(),
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
    pub fn push_op(&mut self, operation: Operation, primitive: Primitive) -> PrimitiveOpId {
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

    /// Create `ObjectDuplicate` containing the same primitive data as `self`. This is needed because
    /// `Object`s can't be cloned as their `id`s must be unique.
    pub fn duplicate(&self) -> ObjectDuplicate {
        let primitive_op_duplicates = self.primitive_op_duplicates();
        ObjectDuplicate {
            id: self.id,
            name: self.name.clone(),
            origin: self.origin,
            primitive_op_duplicates,
        }
    }

    pub fn primitive_op_duplicates(&self) -> Vec<PrimitiveOpDuplicate> {
        self.primitive_ops
            .iter()
            .map(|p_op| p_op.duplicate())
            .collect()
    }

    // Getters

    pub fn id(&self) -> ObjectId {
        self.id
    }

    /// If found, returns a ref to the primitive op and the vec index
    pub fn get_primitive_op(&self, id: PrimitiveOpId) -> Option<(&PrimitiveOp, usize)> {
        self.primitive_ops
            .iter()
            .enumerate()
            .find_map(|(index, prim_op)| {
                if prim_op.id() == id {
                    Some((prim_op, index))
                } else {
                    None
                }
            })
    }

    /// If found, returns a mutable ref to the primitive op and the vec index
    pub fn get_primitive_op_mut(&mut self, id: PrimitiveOpId) -> Option<(&mut PrimitiveOp, usize)> {
        self.primitive_ops
            .iter_mut()
            .enumerate()
            .find_map(|(index, prim_op)| {
                if prim_op.id() == id {
                    Some((prim_op, index))
                } else {
                    None
                }
            })
    }
}

// OBJECT DUPLICATE

/// Contains the same primitive data as an `Object`. This is needed because `Object`s can't be
/// cloned as their `id`s must be unique.
#[derive(Clone)]
pub struct ObjectDuplicate {
    id: ObjectId,
    pub name: String,
    pub origin: Vec3,
    pub primitive_op_duplicates: Vec<PrimitiveOpDuplicate>,
}

impl ObjectDuplicate {
    pub fn id(&self) -> ObjectId {
        self.id
    }
}

impl ObjectDuplicate {
    pub fn encoded_primitive_ops(&self) -> Vec<PrimitiveOpBufferUnit> {
        // avoiding this case should be the responsibility of the functions adding to `primtive_ops`
        debug_assert!(self.primitive_op_duplicates.len() <= MAX_PRIMITIVE_OP_COUNT);

        let mut encoded_primitives = Vec::<PrimitiveOpPacket>::new();
        for primitive_op in &self.primitive_op_duplicates {
            let op_code = primitive_op.op.op_code();
            let primitive_type_code = primitive_op.primitive.type_code();

            let transform = primitive_op.primitive.transform().encoded(self.origin);
            let props = primitive_op.primitive.encoded_props();

            let packet = create_primitive_op_packet(op_code, primitive_type_code, transform, props);
            encoded_primitives.push(packet);
        }
        if self.primitive_op_duplicates.len() == 0 {
            // having no primitive ops would probably break something on the gpu side so lets put a NOP here...
            let packet = nop_primitive_op_packet();
            encoded_primitives.push(packet);
        }

        let mut encoded_object = vec![
            self.id.raw_id() as PrimitiveOpBufferUnit,
            self.primitive_op_duplicates.len() as PrimitiveOpBufferUnit,
        ];
        let encoded_primitives_flattened =
            encoded_primitives.into_iter().flatten().collect::<Vec<_>>();
        encoded_object.extend_from_slice(&encoded_primitives_flattened);
        encoded_object
    }

    pub fn aabb(&self) -> Aabb {
        let mut aabb = Aabb::new_zero();
        for primitive_op in &self.primitive_op_duplicates {
            aabb.union(primitive_op.primitive.aabb());
        }
        aabb.offset(self.origin);
        aabb
    }
}
