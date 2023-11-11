use super::object::{object::ObjectId, primitive_op::PrimitiveOpId};
use glam::DVec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    // camera
    SetCameraLockOn { target_pos: DVec3 },
    UnsetCameraLockOn,
    ResetCamera,

    // objects
    SelectObject(ObjectId),
    DeselectObject(),
    RemoveObject(ObjectId),
    RemoveSelectedObject(),
    CreateAndSelectNewDefaultObject(),

    // primtive op
    SelectPrimitiveOpId(ObjectId, PrimitiveOpId),
    SelectPrimitiveOpIndex(ObjectId, usize),
    DeselectPrimtiveOp(),
    RemoveSelectedPrimitiveOp(),
    RemovePrimitiveOpId(ObjectId, PrimitiveOpId),
    RemovePrimitiveOpIndex(ObjectId, usize),

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

#[derive(Debug, Clone, Copy, PartialEq)]
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
