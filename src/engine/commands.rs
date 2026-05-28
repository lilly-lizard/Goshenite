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

pub fn command_failed_warn(command: Command, failed_because: &str) {
    warn!("command {:?} failed due to: {}", command, failed_because);
}

pub fn failure_warn_collection_error(
    e: CollectionError,
    object_id: ObjectId,
    source_command: Option<Command>,
) {
    match e {
        CollectionError::InvalidId { .. } => {
            failure_warn_invalid_object_id(object_id, source_command)
        }
        CollectionError::OutOfBounds { index, .. } => {
            failure_warn_invalid_primitive_op_index(object_id, index, source_command);
        }
        CollectionError::UniqueIdError(unique_id_error) => {
            failure_warn_unique_id_error(source_command, unique_id_error)
        }
    }
}

pub fn failure_warn_invalid_object_id(object_id: ObjectId, source_command: Option<Command>) {
    if let Some(some_command) = source_command {
        command_failed_warn(some_command, "invalid object id");
    } else {
        warn!(
            "attempted to modify object id {} that doesn't exist in object collection",
            object_id
        );
    }
}

pub fn failure_warn_invalid_primitive_op_index(
    object_id: ObjectId,
    primitive_op_index: PrimitiveOpIndex,
    source_command: Option<Command>,
) {
    if let Some(some_command) = source_command {
        command_failed_warn(some_command, "invalid primitive op index");
    } else {
        warn!(
            "attempted to modify primitive op index {} that doesn't exist in object {}",
            primitive_op_index, object_id
        );
    }
}

pub fn failure_warn_unique_id_error(
    source_command: Option<Command>,
    unique_id_error: UniqueIdError,
) {
    let failed_because = format!(
        "The engine has run out of unique ids to assign to new objects.\
        This case is not yet handled by goshenite!\
        Please report this as a bug...\n
        Returned error: {}",
        unique_id_error
    );
    if let Some(some_command) = source_command {
        command_failed_warn(some_command, &failed_because);
    } else {
        warn!("{}", failed_because);
    }
}
