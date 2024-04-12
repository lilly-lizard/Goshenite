use winit::{
    event::{ElementState, KeyEvent},
    keyboard::{KeyCode, PhysicalKey},
};

/// Records wherever keyboard modifiers are currently held down or not
#[derive(Default)]
pub struct KeyboardModifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
}

impl KeyboardModifiers {
    pub fn reset(&mut self) {
        self.shift = false;
        self.control = false;
        self.alt = false;
    }

    pub fn set(&mut self, key_event: KeyEvent) {
        let PhysicalKey::Code(key_code) = key_event.physical_key else {
            return;
        };

        let modifier_bool = match key_code {
            KeyCode::ShiftLeft => &mut self.shift,
            KeyCode::ShiftRight => &mut self.shift,
            KeyCode::ControlLeft => &mut self.control,
            KeyCode::ControlRight => &mut self.control,
            KeyCode::AltLeft => &mut self.alt,
            KeyCode::AltRight => &mut self.alt,
            _ => return,
        };

        match key_event.state {
            ElementState::Pressed => *modifier_bool = true,
            ElementState::Released => *modifier_bool = false,
        }
    }
}
