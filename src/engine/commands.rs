#![allow(dead_code)]

use crate::{
    engine::object::primitive_op::{PrimitiveOp, PrimitiveOpIndex},
    renderer::config_renderer::RenderDebugOptions,
};

use super::{
    object::{object::ObjectId, operation::Operation},
    primitives::{primitive::Primitive, primitive_transform::PrimitiveTransform},
};
use glam::Vec3;

// ~~ Commands ~~

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    // ~~ Renderer ~~
    SetRenderDebugOptions(RenderDebugOptions),

    // ~~ Save states ~~
    SaveStateCamera,
    LoadStateCamera,
    SaveScene,
    LoadScene,

    // ~~ Camera ~~
    SetCameraLockOnObject(ObjectId),
    UnsetCameraLockOn,
    ResetCamera,
    SetArcballTargetDepth(f64),

    // ~~ Object ~~
    SelectObject(ObjectId),
    DeselectObject(),
    RemoveObject(ObjectId),
    RemoveSelectedObject(),
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
    RemoveSelectedPrimitiveOp(),

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
    UpdatePrimitive {
        object_id: ObjectId,
        primitive_op_index: PrimitiveOpIndex,
        new_primitive: Primitive,
    },
    UpdatePrimitiveTransform {
        object_id: ObjectId,
        primitive_op_index: PrimitiveOpIndex,
        new_transform: PrimitiveTransform,
    },
    UpdateOperation {
        object_id: ObjectId,
        primitive_op_index: PrimitiveOpIndex,
        new_operation: Operation,
    },
    UpdateBlend {
        object_id: ObjectId,
        primitive_op_index: PrimitiveOpIndex,
        new_blend: f32,
    },
    UpdateAlbedo {
        object_id: ObjectId,
        primitive_op_index: PrimitiveOpIndex,
        new_albedo: Vec3,
    },
    UpdateSpecular {
        object_id: ObjectId,
        primitive_op_index: PrimitiveOpIndex,
        new_specular: f32,
    },
    /// Moves a primitive op to a new index in the object's rendering order
    ReOrderPrimitiveOp {
        object_id: ObjectId,
        original_index: PrimitiveOpIndex,
        target_index: PrimitiveOpIndex,
    },

    // ~~ Internal ~~
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

// ~~ Helper Types ~~

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandSource {
    Gui,
    CommandPalette,
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

#[derive(Debug)]
pub enum CommandError {
    InvalidObjectId(ObjectId),
    InvalidPrimitiveOpIndex(ObjectId, PrimitiveOpIndex),
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidObjectId(object_id) => write!(f, "invalid object id {}", object_id),
            Self::InvalidPrimitiveOpIndex(object_id, primitive_op_index) => {
                write!(
                    f,
                    "primitive op index {} not present in object id {}",
                    primitive_op_index, object_id
                )
            }
        }
    }
}

impl std::error::Error for CommandError {}
