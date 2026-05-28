use crate::helper::{more_errors::CollectionError, unique_id_gen::UniqueIdError};

use super::object::{
    object::ObjectId,
    primitive_op::{PrimitiveOp, PrimitiveOpIndex},
};
use glam::Vec3;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

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

// ~~ Failed Command Handling ~~

pub fn failure_warn_collection_error(e: CollectionError, object_id: ObjectId) {
    match e {
        CollectionError::InvalidId { .. } => failure_warn_invalid_object_id(object_id),
        CollectionError::OutOfBounds { index, .. } => {
            failure_warn_invalid_primitive_op_index(object_id, index);
        }
        CollectionError::UniqueIdError(unique_id_error) => {
            failure_warn_unique_id_error(unique_id_error)
        }
    }
}

pub fn failure_warn_invalid_object_id(object_id: ObjectId) {
    warn!(
        "attempted to modify object id {} that doesn't exist in object collection",
        object_id
    );
}

pub fn failure_warn_invalid_primitive_op_index(
    object_id: ObjectId,
    primitive_op_index: PrimitiveOpIndex,
) {
    warn!(
        "attempted to modify primitive op index {} that doesn't exist in object {}",
        primitive_op_index, object_id
    );
}

pub fn failure_warn_unique_id_error(unique_id_error: UniqueIdError) {
    let failed_because = format!(
        "The engine has run out of unique ids to assign to new objects.\
        This case is not yet handled by goshenite!\
        Please report this as a bug...\n
        Returned error: {}",
        unique_id_error
    );
    warn!("{}", failed_because);
}
