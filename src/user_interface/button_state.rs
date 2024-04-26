use super::mouse_button::MouseButton;
use glam::DVec2;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

// ~~ Button State ~~

#[derive(Clone, Copy, PartialEq)]
pub enum ButtonState {
    /// Was down before this frame and now is up.
    /// `start_position` is the cursor position when the button was first pressed down.
    JustReleased { start_position: DVec2 },
    /// Has been up for multiple frames
    UnHeld,
    /// Was up before this frame and now is down.
    /// `start_position` is the cursor position when the button was first pressed down.
    JustClicked { start_position: DVec2 },
    /// Has been down for multple frames.
    /// `start_position` is the cursor position when the button was first pressed down.
    Held { start_position: DVec2 },
}

impl ButtonState {
    pub fn is_just_released(&self) -> bool {
        match self {
            ButtonState::JustReleased { start_position: _ } => true,
            _ => false,
        }
    }

    pub fn is_unheld(&self) -> bool {
        *self == Self::UnHeld
    }

    pub fn is_just_clicked(&self) -> bool {
        match self {
            ButtonState::JustClicked { start_position: _ } => true,
            _ => false,
        }
    }

    pub fn is_held(&self) -> bool {
        match self {
            ButtonState::Held { start_position: _ } => true,
            _ => false,
        }
    }

    pub fn start_position(&self) -> Option<DVec2> {
        match self {
            ButtonState::JustReleased { start_position } => Some(*start_position),
            ButtonState::UnHeld => None,
            ButtonState::JustClicked { start_position } => Some(*start_position),
            ButtonState::Held { start_position } => Some(*start_position),
        }
    }

    #[inline]
    pub fn is_up(&self) -> bool {
        self.is_just_released() || self.is_unheld()
    }

    #[inline]
    pub fn is_down(&self) -> bool {
        self.is_just_clicked() || self.is_held()
    }

    pub fn from_winit_event(
        winit_state: winit::event::ElementState,
        start_position: DVec2,
    ) -> Self {
        match winit_state {
            winit::event::ElementState::Pressed => Self::JustClicked { start_position },
            winit::event::ElementState::Released => Self::JustReleased { start_position },
        }
    }
}

impl Default for ButtonState {
    fn default() -> Self {
        Self::UnHeld
    }
}

// ~~ Mouse Button States ~~

#[derive(Default, Clone, Copy)]
pub struct MouseButtonStates {
    left: ButtonState,
    right: ButtonState,
    middle: ButtonState,
    back: ButtonState,
    forward: ButtonState,

    previous_left: ButtonState,
    previous_right: ButtonState,
    previous_middle: ButtonState,
    previous_back: ButtonState,
    previous_forward: ButtonState,
}

impl MouseButtonStates {
    /// Call this every frame. This is needed to determine if buttons are being held down by
    /// checking the button state from the last time this function was called.
    pub fn increment_frame(&mut self) {
        Self::increment_frame_via_button_pointer(&mut self.left, &mut self.previous_left);
        Self::increment_frame_via_button_pointer(&mut self.right, &mut self.previous_right);
        Self::increment_frame_via_button_pointer(&mut self.middle, &mut self.previous_middle);
        Self::increment_frame_via_button_pointer(&mut self.back, &mut self.previous_back);
        Self::increment_frame_via_button_pointer(&mut self.forward, &mut self.previous_forward);
    }

    fn increment_frame_via_button_pointer(
        current_button: &mut ButtonState,
        previous_button: &mut ButtonState,
    ) {
        let button_update = if !current_button.is_held()
            && current_button.is_down()
            && previous_button.is_down()
        {
            let start_position = current_button
                .start_position()
                .expect("if `is_down` is true, can't be unheld");
            Some(ButtonState::Held { start_position })
        } else if !current_button.is_unheld() && current_button.is_up() && previous_button.is_up() {
            Some(ButtonState::UnHeld)
        } else {
            None
        };
        *previous_button = *current_button;
        if let Some(some_updated_button) = button_update {
            *current_button = some_updated_button;
        }
    }

    pub fn set(
        &mut self,
        button: MouseButton,
        winit_state: winit::event::ElementState,
        cursor_position: DVec2,
    ) {
        let start_position = self.get(button).start_position().unwrap_or(cursor_position);
        let new_state = ButtonState::from_winit_event(winit_state, start_position);

        let (button, previous) = match button {
            MouseButton::Left => (&mut self.left, self.previous_left),
            MouseButton::Right => (&mut self.right, self.previous_right),
            MouseButton::Middle => (&mut self.middle, self.previous_middle),
            MouseButton::Back => (&mut self.back, self.previous_back),
            MouseButton::Forward => (&mut self.forward, self.previous_forward),
        };

        if new_state.is_just_clicked() && previous.is_held() {
            return; // stay held
        }
        *button = new_state;
    }

    pub fn get(&self, button: MouseButton) -> ButtonState {
        match button {
            MouseButton::Left => self.left,
            MouseButton::Right => self.right,
            MouseButton::Middle => self.middle,
            MouseButton::Back => self.back,
            MouseButton::Forward => self.forward,
        }
    }

    pub fn get_previous(&self, button: MouseButton) -> ButtonState {
        match button {
            MouseButton::Left => self.previous_left,
            MouseButton::Right => self.previous_right,
            MouseButton::Middle => self.previous_middle,
            MouseButton::Back => self.previous_back,
            MouseButton::Forward => self.previous_forward,
        }
    }

    #[inline]
    pub fn is_just_released(&self, button: MouseButton) -> bool {
        self.get(button).is_just_released()
    }

    #[inline]
    pub fn is_unheld(&self, button: MouseButton) -> bool {
        self.get(button).is_unheld()
    }

    #[inline]
    pub fn is_just_clicked(&self, button: MouseButton) -> bool {
        self.get(button).is_just_clicked()
    }

    #[inline]
    pub fn is_held(&self, button: MouseButton) -> bool {
        self.get(button).is_held()
    }

    #[inline]
    pub fn is_up(&self, button: MouseButton) -> bool {
        self.get(button).is_up()
    }

    #[inline]
    pub fn is_down(&self, button: MouseButton) -> bool {
        self.get(button).is_down()
    }
}
