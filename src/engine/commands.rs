use super::object::object::ObjectId;
use glam::DVec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    // camera
    SetCameraLockOn { target_pos: DVec3 },
    UnsetCameraLockOn,
    ResetCamera,

    // objects
    RemoveObject(ObjectId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandSource {
    Gui,
    CommandPalette,
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
}
