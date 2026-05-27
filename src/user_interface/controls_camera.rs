use super::{
    keyboard_modifiers::{KeyboardModifier, KeyboardModifierStates},
    mouse_button::MouseButton,
};

// ~~ Camera Control Mouse Mapping ~~

pub const MAX_MODIFIERS: usize = 3;

/// Defines a combination of keyboard modifiers and a mouse button to for controls that require
/// mouse movement e.g. camera control.
#[derive(Default, Clone, Copy)]
pub struct MouseMapping {
    pub mouse_button: MouseButton,
    pub modifiers: [Option<KeyboardModifier>; MAX_MODIFIERS],
}

impl MouseMapping {
    pub fn mapping_active(
        &self,
        button: MouseButton,
        modifier_states: KeyboardModifierStates,
    ) -> bool {
        if button != self.mouse_button {
            return false;
        }
        for modifier in self.modifiers {
            if let Some(some_modifier) = modifier {
                if !modifier_states.is_pressed(some_modifier) {
                    return false;
                }
            }
        }
        true
    }
}

#[derive(Clone, Copy, Debug)]
pub enum CameraAction {
    Look,
    Pan,
    Zoom,
    ArcballTarget,
}

#[derive(Clone, Copy)]
pub struct CameraControlMappings {
    pub look: MouseMapping,
    pub pan: MouseMapping,
    pub zoom: MouseMapping,
    /// To be combined with the scroll wheel to adjust the arcball target. Mapping is disabled if set to `None`
    pub arcball_target_modifier: Option<KeyboardModifier>,
}

impl CameraControlMappings {
    pub fn mapping_active(
        &self,
        action: CameraAction,
        drag_button: MouseButton,
        modifier_states: KeyboardModifierStates,
    ) -> bool {
        match action {
            CameraAction::Look => self.look.mapping_active(drag_button, modifier_states),
            CameraAction::Pan => self.pan.mapping_active(drag_button, modifier_states),
            CameraAction::Zoom => self.zoom.mapping_active(drag_button, modifier_states),
            CameraAction::ArcballTarget => match self.arcball_target_modifier {
                Some(arball_modifier) => modifier_states.is_pressed(arball_modifier),
                None => false,
            },
        }
    }
}

impl Default for CameraControlMappings {
    fn default() -> Self {
        Self {
            look: MouseMapping {
                mouse_button: MouseButton::Left,
                ..Default::default()
            },
            pan: MouseMapping {
                mouse_button: MouseButton::Right,
                ..Default::default()
            },
            zoom: MouseMapping {
                mouse_button: MouseButton::Middle,
                ..Default::default()
            },
            arcball_target_modifier: Some(KeyboardModifier::Shift),
        }
    }
}
