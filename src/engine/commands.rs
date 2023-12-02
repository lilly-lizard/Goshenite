use super::{
    object::{object::ObjectId, operation::Operation, primitive_op::PrimitiveOpId},
    primitives::{primitive::Primitive, primitive_transform::PrimitiveTransform},
};
use glam::{DVec3, Vec3};

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    // camera
    SetCameraLockOnPos(DVec3),
    SetCameraLockOnObject(ObjectId),
    UnsetCameraLockOn,
    ResetCamera,

    // object
    SelectObject(ObjectId),
    DeselectObject(),
    RemoveObject(ObjectId),
    RemoveSelectedObject(),
    CreateAndSelectNewDefaultObject(),
    SetObjectOrigin {
        object_id: ObjectId,
        origin: Vec3,
    },
    SetObjectName {
        object_id: ObjectId,
        new_name: String,
    },

    // primtive op - selection
    SelectPrimitiveOpId(ObjectId, PrimitiveOpId),
    SelectPrimitiveOpIndex(ObjectId, usize),
    DeselectPrimtiveOp(),

    // primitive op - remove
    RemoveSelectedPrimitiveOp(),
    RemovePrimitiveOpId(ObjectId, PrimitiveOpId),
    RemovePrimitiveOpIndex(ObjectId, usize),

    // primitive op - push
    PushOp {
        object_id: ObjectId,
        operation: Operation,
        primitive: Primitive,
    },
    PushOpAndSelect {
        object_id: ObjectId,
        operation: Operation,
        primitive: Primitive,
    },

    // primitive op - modify
    SetPrimitiveOp {
        object_id: ObjectId,
        primitive_op_id: PrimitiveOpId,
        new_primitive: Primitive,
        new_transform: PrimitiveTransform,
        new_operation: Operation,
    },
    SetPrimitive {
        object_id: ObjectId,
        primitive_op_id: PrimitiveOpId,
        new_primitive: Primitive,
    },
    SetPrimitiveTransform {
        object_id: ObjectId,
        primitive_op_id: PrimitiveOpId,
        new_transform: PrimitiveTransform,
    },
    SetOperation {
        object_id: ObjectId,
        primitive_op_id: PrimitiveOpId,
        new_operation: Operation,
    },
    ShiftPrimitiveOps {
        object_id: ObjectId,
        source_index: usize,
        target_index: usize,
    },

    // internal
    Validate(ValidationCommand),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ValidationCommand {
    SelectedObject(),
}

impl From<ValidationCommand> for Command {
    fn from(v_command: ValidationCommand) -> Self {
        Self::Validate(v_command)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandSource {
    Gui,
    CommandPalette,
    // https://docs.rs/keyboard-types/latest/keyboard_types/struct.ShortcutMatcher.html
    KeyboardShortcut,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommandWithSource {
    pub command: Command,
    pub source: CommandSource,
}

impl CommandWithSource {
    pub fn new_from_gui(command: Command) -> Self {
        Self {
            command,
            source: CommandSource::Gui,
        }
    }

    pub fn new_from_palette(command: Command) -> Self {
        Self {
            command,
            source: CommandSource::CommandPalette,
        }
    }

    pub fn new_from_shortcut(command: Command) -> Self {
        Self {
            command,
            source: CommandSource::KeyboardShortcut,
        }
    }
}

// ~~ Errors ~~

#[derive(Debug, Clone, Copy)]
pub enum CommandError {
    InvalidObjectId(ObjectId),
    InvalidPrimitiveOpId(ObjectId, PrimitiveOpId),
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::InvalidObjectId(object_id) => write!(f, "invalid object id {}", object_id),
            Self::InvalidPrimitiveOpId(object_id, primitive_op_id) => {
                write!(
                    f,
                    "primitive op id {} not present in object id {}",
                    primitive_op_id, object_id
                )
            }
        }
    }
}

impl std::error::Error for CommandError {}
