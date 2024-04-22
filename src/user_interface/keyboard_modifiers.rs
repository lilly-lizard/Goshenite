use crate::engine::settings::{SETTING_NAME_ALT, SETTING_NAME_CONTROL, SETTING_NAME_SHIFT};
use std::fmt::Display;
use winit::{
    event::{ElementState, KeyEvent},
    keyboard::{KeyCode, PhysicalKey},
};

// ~~ Keyboard Modifier ~~

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum KeyboardModifier {
    Shift,
    Control,
    Alt,
}

impl KeyboardModifier {
    pub fn from_setting_name(setting_name: &str) -> Option<Self> {
        match setting_name {
            SETTING_NAME_SHIFT => Some(Self::Shift),
            SETTING_NAME_CONTROL => Some(Self::Control),
            SETTING_NAME_ALT => Some(Self::Alt),
            _ => None,
        }
    }

    pub fn setting_name(&self) -> &'static str {
        match *self {
            Self::Shift => &SETTING_NAME_SHIFT,
            Self::Control => &SETTING_NAME_CONTROL,
            Self::Alt => &SETTING_NAME_ALT,
        }
    }
}

impl Display for KeyboardModifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.setting_name())
    }
}

// ~~ Keyboard Modifier States ~~

/// A modifier is `true` if it currently held down.
#[derive(Default)]
pub struct KeyboardModifierStates {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
}

impl KeyboardModifierStates {
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

    pub fn is_pressed(&self, modifier: KeyboardModifier) -> bool {
        match modifier {
            KeyboardModifier::Shift => self.shift,
            KeyboardModifier::Control => self.control,
            KeyboardModifier::Alt => self.alt,
        }
    }
}
