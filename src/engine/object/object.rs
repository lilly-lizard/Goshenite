use super::{
    operation::Operation,
    primitive_op::{PrimitiveOp, PrimitiveOpId},
};
use crate::{
    engine::{
        aabb::Aabb,
        primitives::{
            primitive::{EncodablePrimitive, Primitive},
            primitive_transform::PrimitiveTransform,
        },
    },
    helper::{
        more_errors::CollectionError,
        unique_id_gen::{UniqueId, UniqueIdError, UniqueIdGen, UniqueIdType},
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

impl UniqueIdType for ObjectId {
    fn raw_id(&self) -> UniqueId {
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

#[derive(Debug)]
pub struct Object {
    id: ObjectId,
    pub name: String,
    pub origin: Vec3,
    pub primitive_ops: Vec<PrimitiveOp>,

    primitive_op_id_gen: UniqueIdGen<PrimitiveOpId>,
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

    /// Returns the index of the removed primitive op
    pub fn remove_primitive_op_id(
        &mut self,
        primitive_op_id: PrimitiveOpId,
    ) -> Result<usize, CollectionError> {
        let index_res = self
            .primitive_ops
            .iter()
            .position(|primitive_op| primitive_op.id() == primitive_op_id);

        let _ = self.primitive_op_id_gen.recycle_id(primitive_op_id);

        if let Some(index) = index_res {
            self.primitive_ops.remove(index);
            return Ok(index);
        } else {
            return Err(CollectionError::InvalidId {
                raw_id: primitive_op_id.raw_id(),
            });
        }
    }

    pub fn remove_primitive_op_index(
        &mut self,
        index: usize,
    ) -> Result<PrimitiveOpId, CollectionError> {
        let op_count = self.primitive_ops.len();
        if index >= op_count {
            return Err(CollectionError::OutOfBounds {
                index,
                size: op_count,
            });
        }

        let removed_prim_op = self.primitive_ops.remove(index);

        let _ = self.primitive_op_id_gen.recycle_id(removed_prim_op.id());
        Ok(removed_prim_op.id())
    }

    /// Returns the id of the newly created primitive op
    pub fn push_op(
        &mut self,
        operation: Operation,
        primitive: Primitive,
        transform: PrimitiveTransform,
    ) -> Result<PrimitiveOpId, UniqueIdError> {
        let primitive_op_id = self.primitive_op_id_gen.new_id()?;

        self.primitive_ops.push(PrimitiveOp::new(
            primitive_op_id,
            operation,
            primitive,
            transform,
        ));
        Ok(primitive_op_id)
    }

    pub fn shift_primitive_ops(
        &mut self,
        source_index: usize,
        target_index: usize,
    ) -> Result<(), ShiftSliceError> {
        shift_slice(source_index, target_index, &mut self.primitive_ops)
    }

    /// Create `ObjectSnapshot` containing the same primitive data as `self`. This is needed because
    /// `Object`s can't be cloned as their `id`s must be unique.
    pub fn duplicate(&self) -> ObjectSnapshot {
        ObjectSnapshot {
            name: self.name.clone(),
            origin: self.origin,
            primitive_ops: self.primitive_ops.clone(),
        }
    }

    // Getters

    pub fn id(&self) -> ObjectId {
        self.id
    }

    pub fn get_primitive_op(&self, primitive_op_id: PrimitiveOpId) -> Option<&PrimitiveOp> {
        self.primitive_ops.iter().find_map(|primitive_op| {
            if primitive_op.id() == primitive_op_id {
                Some(primitive_op)
            } else {
                None
            }
        })
    }

    pub fn get_primitive_op_mut(
        &mut self,
        primitive_op_id: PrimitiveOpId,
    ) -> Option<&mut PrimitiveOp> {
        self.primitive_ops.iter_mut().find_map(|primitive_op| {
            if primitive_op.id() == primitive_op_id {
                Some(primitive_op)
            } else {
                None
            }
        })
    }

    pub fn get_primitive_op_with_index(
        &self,
        primitive_op_id: PrimitiveOpId,
    ) -> Option<(&PrimitiveOp, usize)> {
        self.primitive_ops
            .iter()
            .enumerate()
            .find_map(|(index, primitive_op)| {
                if primitive_op.id() == primitive_op_id {
                    Some((primitive_op, index))
                } else {
                    None
                }
            })
    }

    pub fn set_primitive_op(
        &mut self,
        primitive_op_id: PrimitiveOpId,
        new_primitive: Option<Primitive>,
        new_transform: Option<PrimitiveTransform>,
        new_operation: Option<Operation>,
    ) -> Result<(), CollectionError> {
        let primitive_op_search_res = self.get_primitive_op_mut(primitive_op_id);

        let primitive_op_ref = match primitive_op_search_res {
            Some(p) => p,
            None => {
                return Err(CollectionError::InvalidId {
                    raw_id: primitive_op_id.raw_id(),
                })
            }
        };

        if let Some(some_new_primitive) = new_primitive {
            primitive_op_ref.primitive = some_new_primitive;
        }
        if let Some(some_new_transform) = new_transform {
            primitive_op_ref.primitive_transform = some_new_transform;
        }
        if let Some(some_new_operation) = new_operation {
            primitive_op_ref.op = some_new_operation;
        }
        return Ok(());
    }
}

// ~~ Object Snapshot ~~

/// Contains the same primitive data as an `Object`.
#[derive(Clone)]
pub struct ObjectSnapshot {
    pub name: String,
    pub origin: Vec3,
    pub primitive_ops: Vec<PrimitiveOp>,
}

impl ObjectSnapshot {
    pub fn encoded_primitive_ops(&self, object_id: ObjectId) -> Vec<PrimitiveOpBufferUnit> {
        // avoiding this case should be the responsibility of the functions adding to `primtive_ops`
        debug_assert!(self.primitive_ops.len() <= MAX_PRIMITIVE_OP_COUNT);

        let mut encoded_primitives = Vec::<PrimitiveOpPacket>::new();
        for primitive_op in &self.primitive_ops {
            let primitive_op = primitive_op;
            let primitive = primitive_op.primitive;

            let encoded_op_code = primitive_op.op.op_code();
            let encoded_transform = primitive_op.primitive_transform.gpu_encoded(self.origin);
            let encoded_props = primitive.encoded_props();

            let packet =
                create_primitive_op_packet(encoded_op_code, encoded_transform, encoded_props);
            encoded_primitives.push(packet);
        }
        if self.primitive_ops.len() == 0 {
            // having no primitive ops would probably break something on the gpu side so lets put a NOP here...
            let packet = nop_primitive_op_packet();
            encoded_primitives.push(packet);
        }

        let mut encoded_object = vec![
            object_id.raw_id() as PrimitiveOpBufferUnit,
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
            aabb.union(
                primitive_op
                    .primitive
                    .aabb(primitive_op.primitive_transform),
            );
        }
        aabb.offset(self.origin);
        aabb
    }
}
