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
        PrimitiveOpPacket, MAX_PRIMITIVE_OP_COUNT,
    },
};
use egui_dnd::utils::{shift_slice, ShiftSliceError};
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
    pub origin: Vec3,
    pub primitive_ops: Vec<PrimitiveOp>,

    primitive_op_id_gen: UniqueIdGen<PrimitiveOpId>,
}

impl Object {
    pub fn new(name: String, origin: Vec3) -> Self {
        Self {
            name,
            origin,
            primitive_ops: Vec::new(),
            primitive_op_id_gen: UniqueIdGen::new(),
        }
    }

    /// Returns the index of the removed primitive op
    pub fn remove_primitive_op_id(
        &mut self,
        remove_primitive_op_id: PrimitiveOpId,
    ) -> Result<usize, CollectionError> {
        let index_res = self
            .primitive_ops
            .iter()
            .position(|check_primitive_op| check_primitive_op.id() == remove_primitive_op_id);

        _ = self.primitive_op_id_gen.recycle_id(remove_primitive_op_id);

        if let Some(index) = index_res {
            self.primitive_ops.remove(index);
            return Ok(index);
        } else {
            return Err(CollectionError::InvalidId {
                raw_id: remove_primitive_op_id.raw_id(),
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

        _ = self.primitive_op_id_gen.recycle_id(removed_prim_op.id());
        Ok(removed_prim_op.id())
    }

    /// Returns the id of the newly created primitive op
    pub fn push_primitive_op(
        &mut self,
        primitive: Primitive,
        transform: PrimitiveTransform,
        op: Operation,
        blend: f32,
        albedo: Vec3,
        specular: f32,
    ) -> Result<PrimitiveOpId, UniqueIdError> {
        let primitive_op_id = self.primitive_op_id_gen.new_id()?;
        self.primitive_ops.push(PrimitiveOp::new(
            primitive_op_id,
            primitive,
            transform,
            op,
            blend,
            albedo,
            specular,
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

    // Getters

    pub fn get_primitive_op(&self, get_primitive_op_id: PrimitiveOpId) -> Option<&PrimitiveOp> {
        self.primitive_ops.iter().find_map(|check_primitive_op| {
            if check_primitive_op.id() == get_primitive_op_id {
                Some(check_primitive_op)
            } else {
                None
            }
        })
    }

    pub fn get_primitive_op_mut(
        &mut self,
        get_primitive_op_id: PrimitiveOpId,
    ) -> Option<&mut PrimitiveOp> {
        self.primitive_ops
            .iter_mut()
            .find_map(|check_primitive_op| {
                if check_primitive_op.id() == get_primitive_op_id {
                    Some(check_primitive_op)
                } else {
                    None
                }
            })
    }

    pub fn get_primitive_op_and_index(
        &self,
        get_primitive_op_id: PrimitiveOpId,
    ) -> Option<(&PrimitiveOp, usize)> {
        self.primitive_ops
            .iter()
            .enumerate()
            .find_map(|(index, check_primitive_op)| {
                if check_primitive_op.id() == get_primitive_op_id {
                    Some((check_primitive_op, index))
                } else {
                    None
                }
            })
    }

    pub fn set_primitive_op_id(
        &mut self,
        primitive_op_id: PrimitiveOpId,
        new_primitive: Option<Primitive>,
        new_transform: Option<PrimitiveTransform>,
        new_operation: Option<Operation>,
        new_blend: Option<f32>,
        new_albedo: Option<Vec3>,
        new_specular: Option<f32>,
    ) -> Result<(), CollectionError> {
        let primitive_op_search_res = self.get_primitive_op_mut(primitive_op_id);
        let Some(primitive_op_ref) = primitive_op_search_res else {
            return Err(CollectionError::InvalidId {
                raw_id: primitive_op_id.raw_id(),
            });
        };
        set_primitive_op_internal(
            primitive_op_ref,
            new_primitive,
            new_transform,
            new_operation,
            new_blend,
            new_albedo,
            new_specular,
        )
    }

    pub fn set_primitive_op_index(
        &mut self,
        primitive_op_index: usize,
        new_primitive: Option<Primitive>,
        new_transform: Option<PrimitiveTransform>,
        new_operation: Option<Operation>,
        new_blend: Option<f32>,
        new_albedo: Option<Vec3>,
        new_specular: Option<f32>,
    ) -> Result<(), CollectionError> {
        let primitive_op_search_res = self.primitive_ops.get_mut(primitive_op_index);
        let Some(primitive_op_ref) = primitive_op_search_res else {
            return Err(CollectionError::OutOfBounds {
                index: primitive_op_index,
                size: self.primitive_ops.len(),
            });
        };
        set_primitive_op_internal(
            primitive_op_ref,
            new_primitive,
            new_transform,
            new_operation,
            new_blend,
            new_albedo,
            new_specular,
        )
    }

    pub fn encoded_primitive_ops(&self, object_id: ObjectId) -> Vec<PrimitiveOpBufferUnit> {
        // avoiding this case should be the responsibility of the functions adding to `primtive_ops`
        debug_assert!(self.primitive_ops.len() <= MAX_PRIMITIVE_OP_COUNT);

        let mut encoded_primitives = Vec::<PrimitiveOpPacket>::new();
        for primitive_op in &self.primitive_ops {
            let packet = create_primitive_op_packet(primitive_op, self.origin);
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
        aabb.offset(self.origin);
        aabb
    }
}

fn set_primitive_op_internal(
    primitive_op_ref: &mut PrimitiveOp,
    new_primitive: Option<Primitive>,
    new_transform: Option<PrimitiveTransform>,
    new_operation: Option<Operation>,
    new_blend: Option<f32>,
    new_albedo: Option<Vec3>,
    new_specular: Option<f32>,
) -> Result<(), CollectionError> {
    if let Some(some_new_primitive) = new_primitive {
        primitive_op_ref.primitive = some_new_primitive;
    }
    if let Some(some_new_transform) = new_transform {
        primitive_op_ref.transform = some_new_transform;
    }
    if let Some(some_new_operation) = new_operation {
        primitive_op_ref.op = some_new_operation;
    }
    if let Some(some_new_blend) = new_blend {
        primitive_op_ref.blend = some_new_blend;
    }
    if let Some(some_new_albedo) = new_albedo {
        primitive_op_ref.albedo = some_new_albedo;
    }
    if let Some(some_new_specular) = new_specular {
        primitive_op_ref.specular = some_new_specular;
    }
    return Ok(());
}
