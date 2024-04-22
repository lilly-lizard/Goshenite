use crate::engine::settings::{
    SETTING_NAME_MOUSE_BACK, SETTING_NAME_MOUSE_FORWARD, SETTING_NAME_MOUSE_LEFT,
    SETTING_NAME_MOUSE_MIDDLE, SETTING_NAME_MOUSE_RIGHT,
};

// ~~ Mouse Button ~~

/// Mouse buttons supported by engine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
}

/// List of available [`MouseButton`] enum variations. Note that the order affects the priority for things like dragging logic.
pub static MOUSE_BUTTONS: [MouseButton; 5] = [
    MouseButton::Left,
    MouseButton::Right,
    MouseButton::Middle,
    MouseButton::Back,
    MouseButton::Forward,
];

impl MouseButton {
    pub fn from_winit(button: winit::event::MouseButton) -> Result<Self, String> {
        match button {
            winit::event::MouseButton::Left => Ok(Self::Left),
            winit::event::MouseButton::Right => Ok(Self::Right),
            winit::event::MouseButton::Middle => Ok(Self::Middle),
            winit::event::MouseButton::Back => Ok(Self::Back),
            winit::event::MouseButton::Forward => Ok(Self::Forward),
            winit::event::MouseButton::Other(code) => match code {
                _ => Err(format!(
                    "attempted to index unsupported mouse button code: {}",
                    code
                )),
            },
        }
    }

    pub fn from_setting_name(setting_name: &str) -> Option<Self> {
        match setting_name {
            SETTING_NAME_MOUSE_LEFT => Some(Self::Left),
            SETTING_NAME_MOUSE_RIGHT => Some(Self::Right),
            SETTING_NAME_MOUSE_MIDDLE => Some(Self::Middle),
            SETTING_NAME_MOUSE_BACK => Some(Self::Back),
            SETTING_NAME_MOUSE_FORWARD => Some(Self::Forward),
            _ => None,
        }
    }
}

impl Default for MouseButton {
    fn default() -> Self {
        Self::Left
    }
}
