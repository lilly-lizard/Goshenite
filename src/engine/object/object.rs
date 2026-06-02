use super::{operation::Operation, primitive_op::PrimitiveOp};
use crate::{
    engine::{
        aabb::Aabb,
        object::primitive_op::PrimitiveOpIndex,
        primitives::{
            primitive::{EncodablePrimitive, Primitive},
            transform::{ObjectInstances, Transform},
        },
    },
    helper::{
        shift_slice::{shift_slice, ShiftSliceError},
        unique_id_gen::{UniqueId, UniqueIdType},
    },
    renderer::shader_interfaces::primitive_op_buffer::{
        create_primitive_op_packet, nop_primitive_op_packet, PrimitiveOpBufferUnit,
        PrimitiveOpPacket, MAX_PRIMITIVE_OP_COUNT,
    },
};
use glam::Vec3;
use serde::{Deserialize, Serialize};

// ~~ Object Id ~~

#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize,
)]
pub struct ObjectId(UniqueId);

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

// ~~ Object ~~

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Object {
    pub name: String,
    pub center: Vec3,
    pub primitive_ops: Vec<PrimitiveOp>,
    pub instances: ObjectInstances,
}

impl Object {
    pub fn new(name: String, center: Vec3) -> Self {
        Self {
            name,
            center,
            primitive_ops: Vec::new(),
            instances: ObjectInstances::Single,
        }
    }

    /// Returns the id of the newly created primitive op
    pub fn push_primitive_op(
        &mut self,
        primitive: Primitive,
        transform: Transform,
        op: Operation,
        blend: f32,
        albedo: Vec3,
        specular: f32,
    ) {
        self.primitive_ops.push(PrimitiveOp::new(
            primitive, transform, op, blend, albedo, specular,
        ));
    }

    pub fn shift_primitive_ops(
        &mut self,
        source_index: PrimitiveOpIndex,
        target_index: PrimitiveOpIndex,
    ) -> Result<(), ShiftSliceError> {
        shift_slice(source_index, target_index, &mut self.primitive_ops)
    }

    // Setters

    pub fn encoded_primitive_ops(&self, object_id: ObjectId) -> Vec<PrimitiveOpBufferUnit> {
        // avoiding this case should be the responsibility of the functions adding to `primtive_ops`
        debug_assert!(self.primitive_ops.len() <= MAX_PRIMITIVE_OP_COUNT);

        let mut encoded_primitives = Vec::<PrimitiveOpPacket>::new();
        for primitive_op in &self.primitive_ops {
            let packet = create_primitive_op_packet(primitive_op);
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
        let encoded_primitives_flattened: Vec<u32> =
            encoded_primitives.into_iter().flatten().collect();
        encoded_object.extend_from_slice(&encoded_primitives_flattened);
        encoded_object
    }

    pub fn aabb(&self) -> Aabb {
        let mut aabb = Aabb::new_zero();
        for primitive_op in &self.primitive_ops {
            aabb.union(primitive_op.primitive.aabb(primitive_op.transform));
        }
        aabb
    }
}
