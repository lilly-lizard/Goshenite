use super::mouse_button::MouseButton;

// ~~ Button State ~~

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    /// Was down before this frame and now is up
    JustReleased,
    /// Has been up for multiple frames
    UnHeld,
    /// Was up before this frame and now is down
    JustClicked,
    /// Has been down for multple frames
    Held,
}

impl ButtonState {
    pub fn is_released(&self) -> bool {
        *self == Self::JustReleased
    }

    pub fn is_unheld(&self) -> bool {
        *self == Self::UnHeld
    }

    pub fn is_clicked(&self) -> bool {
        *self == Self::JustClicked
    }

    pub fn is_held(&self) -> bool {
        *self == Self::Held
    }

    #[inline]
    pub fn is_up(&self) -> bool {
        self.is_released() || self.is_unheld()
    }

    #[inline]
    pub fn is_down(&self) -> bool {
        self.is_clicked() || self.is_held()
    }

    pub fn from_winit_event(winit_state: winit::event::ElementState) -> Self {
        match winit_state {
            winit::event::ElementState::Pressed => Self::JustClicked,
            winit::event::ElementState::Released => Self::JustReleased,
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
        let button_update = if current_button.is_down() && previous_button.is_down() {
            Some(ButtonState::Held)
        } else if current_button.is_up() && previous_button.is_up() {
            Some(ButtonState::UnHeld)
        } else {
            None
        };
        *previous_button = *current_button;
        if let Some(some_updated_button) = button_update {
            *current_button = some_updated_button;
        }
    }

    pub fn set(&mut self, button: MouseButton, winit_state: winit::event::ElementState) {
        let state = ButtonState::from_winit_event(winit_state);
        let (button, previous) = match button {
            MouseButton::Left => (&mut self.left, self.previous_left),
            MouseButton::Right => (&mut self.right, self.previous_right),
            MouseButton::Middle => (&mut self.middle, self.previous_middle),
            MouseButton::Back => (&mut self.back, self.previous_back),
            MouseButton::Forward => (&mut self.forward, self.previous_forward),
        };
        Self::set_via_button_pointer(button, previous, state)
    }

    #[inline]
    fn set_via_button_pointer(button: &mut ButtonState, previous: ButtonState, new: ButtonState) {
        if new.is_clicked() && previous.is_held() {
            // stay held
            return;
        }
        *button = new;
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
    pub fn is_released(&self, button: MouseButton) -> bool {
        self.get(button).is_released()
    }

    #[inline]
    pub fn is_unheld(&self, button: MouseButton) -> bool {
        self.get(button).is_unheld()
    }

    #[inline]
    pub fn is_clicked(&self, button: MouseButton) -> bool {
        self.get(button).is_clicked()
    }

    #[inline]
    pub fn is_held(&self, button: MouseButton) -> bool {
        self.get(button).is_held()
    }
}
