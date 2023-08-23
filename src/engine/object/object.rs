use super::{
    operation::Operation,
    primitive_op::{PrimitiveOp, PrimitiveOpDuplicate, PrimitiveOpId},
};
use crate::{
    engine::{aabb::Aabb, primitives::primitive::Primitive},
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
use std::hash::{Hash, Hasher};

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
    name: String,
    origin: Vec3,
    primitive_ops: Vec<PrimitiveOp>,

    primitive_op_id_gen: UniqueIdGen,
}

impl Object {
    pub fn new(id: ObjectId, name: String, origin: Vec3) -> Self {
        let mut primitive_op_id_gen = UniqueIdGen::new();
        let new_raw_id = primitive_op_id_gen
            .new_id()
            .expect("todo should probably handle this somehow...");

        Self {
            id,
            name,
            origin,
            primitive_ops: vec![],
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
    pub fn push_op(
        &mut self,
        operation: Operation,
        primitive: Box<dyn Primitive>,
    ) -> PrimitiveOpId {
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
            let primitive_type_code = primitive_op.primitive.type_code();

            let transform = primitive_op.primitive.transform().encoded(self.origin);
            let props = primitive_op.primitive.encoded_props();

            let packet = create_primitive_op_packet(op_code, primitive_type_code, transform, props);
            encoded_primitives.push(packet);
        }
        if self.primitive_ops.len() == 0 {
            // having no primitive ops would probably break something on the gpu side so lets put a NOP here...
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
            aabb.union(primitive_op.primitive.aabb());
        }
        aabb.offset(self.origin);
        aabb
    }

    pub fn set_origin(&mut self, origin: Vec3) {
        self.origin = origin;
    }

    pub fn duplicate(&self) -> ObjectDuplicate {
        let primitive_op_duplicates = self
            .primitive_ops
            .iter()
            .map(|p_op| p_op.duplicate())
            .collect();

        ObjectDuplicate {
            id: self.id,
            name: self.name,
            origin: self.origin,
            primitive_ops: primitive_op_duplicates,
        }
    }

    // Getters

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

// CLONED OBJECT

// todo clone box trait rabbit-hole...
// - https://users.rust-lang.org/t/solved-is-it-possible-to-clone-a-boxed-trait-object/1714/7
// - https://stackoverflow.com/questions/53987976/what-does-a-trait-requiring-sized-have-to-do-with-being-unable-to-have-trait-obj
// - https://web.mit.edu/rust-lang_v1.25/arch/amd64_ubuntu1404/share/doc/rust/html/book/first-edition/trait-objects.html
pub struct ObjectDuplicate {
    pub id: ObjectId,
    pub name: String,
    pub origin: Vec3,
    pub primitive_ops: Vec<PrimitiveOpDuplicate>,
}

impl Hash for ObjectDuplicate {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // ids should be unique so we can just hash this.
        self.id.hash(state);
    }
}
impl PartialEq for ObjectDuplicate {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for ObjectDuplicate {}
