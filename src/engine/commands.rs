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

    // primtive op
    SelectPrimitiveOpId(PrimitiveOpId),
    SelectPrimitiveOpIndex(usize),
    DeselectPrimtiveOp(),
    RemovePrimitiveOpId(PrimitiveOpId),
    RemovePrimitiveOpIndex(usize),
    RemoveSelectedPrimitiveOp(),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandSource {
    Gui,
    CommandPalette,
    // todo https://docs.rs/keyboard-types/latest/keyboard_types/struct.ShortcutMatcher.html
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
