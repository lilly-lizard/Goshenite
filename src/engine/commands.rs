use crate::renderer::config_renderer::RenderOptions;

use super::{
    object::{object::ObjectId, operation::Operation, primitive_op::PrimitiveOpId},
    primitives::{primitive::Primitive, primitive_transform::PrimitiveTransform},
};
use glam::{DVec3, Vec3};

// ~~ Commands ~~

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    // ~~ Renderer ~~
    SetRenderOptions(RenderOptions),

    // ~~ Save states ~~
    SaveStateCamera,
    LoadStateCamera,
    SaveAllObjects,
    LoadObjects,

    // ~~ Camera ~~
    SetCameraLockOnPos(DVec3),
    SetCameraLockOnObject(ObjectId),
    UnsetCameraLockOn,
    ResetCamera,

    // ~~ Object ~~
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

    // ~~ Primtive Op: Selection ~~
    SelectPrimitiveOp(TargetPrimitiveOp),
    DeselectPrimtiveOp(),

    // ~~ Primitive Op: Remove ~~
    RemovePrimitiveOp(TargetPrimitiveOp),

    // ~~ Primitive Op: Push ~~
    PushPrimitiveOp {
        object_id: ObjectId,
        primitive: Primitive,
        transform: PrimitiveTransform,
        operation: Operation,
        blend: f32,
        albedo: Vec3,
        specular: f32,
    },
    PushPrimitiveOpAndSelect {
        object_id: ObjectId,
        primitive: Primitive,
        transform: PrimitiveTransform,
        operation: Operation,
        blend: f32,
        albedo: Vec3,
        specular: f32,
    },

    // ~~ Primitive Op: Modify ~~
    SetPrimitiveOp {
        target_primitive_op: TargetPrimitiveOp,
        new_primitive: Primitive,
        new_transform: PrimitiveTransform,
        new_operation: Operation,
        new_blend: f32,
        new_albedo: Vec3,
        new_specular: f32,
    },
    SetPrimitive {
        target_primitive_op: TargetPrimitiveOp,
        new_primitive: Primitive,
    },
    SetPrimitiveTransform {
        target_primitive_op: TargetPrimitiveOp,
        new_transform: PrimitiveTransform,
    },
    SetOperation {
        target_primitive_op: TargetPrimitiveOp,
        new_operation: Operation,
    },
    SetBlend {
        target_primitive_op: TargetPrimitiveOp,
        new_blend: f32,
    },
    SetAlbedo {
        target_primitive_op: TargetPrimitiveOp,
        new_albedo: Vec3,
    },
    SetSpecular {
        target_primitive_op: TargetPrimitiveOp,
        new_specular: f32,
    },
    ShiftPrimitiveOps {
        object_id: ObjectId,
        source_index: usize,
        target_index: usize,
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TargetPrimitiveOp {
    Selected,
    Id(ObjectId, PrimitiveOpId),
    Index(ObjectId, usize),
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

#[derive(Debug)]
pub enum CommandError {
    InvalidObjectId(ObjectId),
    InvalidPrimitiveOpId(ObjectId, PrimitiveOpId),
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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
