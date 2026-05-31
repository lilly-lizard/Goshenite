use crate::engine::primitives::transform::ObjectInstances;

use super::object::{
    object::ObjectId,
    primitive_op::{PrimitiveOp, PrimitiveOpIndex},
};
use glam::Vec3;

// ~~ Commands ~~

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    // ~~ Save states ~~
    SaveStateCamera,
    LoadStateCamera,
    SaveScene,
    LoadScene,

    // ~~ Object ~~
    SelectObject(ObjectId),
    #[allow(unused)]
    DeselectObject(),
    RemoveObject(ObjectId),
    CreateAndSelectNewDefaultObject(),
    SetObjectCenter {
        object_id: ObjectId,
        center: Vec3,
    },
    SetObjectName {
        object_id: ObjectId,
        new_name: String,
    },
    SetObjectInstances {
        object_id: ObjectId,
        new_instances: ObjectInstances,
    },

    // ~~ Primtive Op: Selection ~~
    SelectPrimitiveOp(ObjectId, PrimitiveOpIndex),
    DeselectPrimtiveOp(),

    // ~~ Primitive Op: Remove ~~
    RemovePrimitiveOp(ObjectId, PrimitiveOpIndex),

    // ~~ Primitive Op: Push ~~
    PushPrimitiveOp {
        object_id: ObjectId,
        primitive_op: PrimitiveOp,
    },
    PushPrimitiveOpAndSelect {
        object_id: ObjectId,
        primitive_op: PrimitiveOp,
    },

    // ~~ Primitive Op: Modify ~~
    UpdatePrimitiveOp {
        object_id: ObjectId,
        primitive_op_index: PrimitiveOpIndex,
        new_primitive_op: PrimitiveOp,
    },
    /// Moves a primitive op to a new index in the object's rendering order
    ReOrderPrimitiveOp {
        object_id: ObjectId,
        original_index: PrimitiveOpIndex,
        target_index: PrimitiveOpIndex,
    },

    // ~~ Internal ~~
    ValidateSelectedObject,
}
