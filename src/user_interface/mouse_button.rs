// ~~ Mouse Button ~~

/// Mouse buttons supported by engine
#[derive(Debug, Clone, Copy, PartialEq)]
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
}

impl Default for MouseButton {
    fn default() -> Self {
        Self::Left
    }
}
