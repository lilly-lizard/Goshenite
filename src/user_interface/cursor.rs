use super::{
    button_state::{ButtonState, MouseButtonStates},
    mouse_button::{MouseButton, MOUSE_BUTTONS},
};
use glam::DVec2;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use winit::event::MouseScrollDelta;

/// Records and processes the state of the mouse cursor
pub struct Cursor {
    /// Describes wherver the cursor is currently within the window bounds
    in_window: bool,
    /// The current cursor position. None if the cursor position is unknown
    /// (waiting for first [`WindowEvent::CursorMoved`](winit::event::WindowEvent::CursorMoved) event)
    position: Option<DVec2>,
    /// The cursor position in the previous frame. Used to calculate [`Cursor::position_frame_change`]
    position_previous: Option<DVec2>,
    /// The change in cursor position since the previous frame
    position_frame_change: DVec2,
    /// Describes wherever each mouse button is pressed
    mouse_button_states: MouseButtonStates,
    /// Horizontal/vertical scrolling since last call to [`get_and_clear_scroll_delta`](Self::get_and_clear_scroll_delta).
    scroll_delta: DVec2,
    /// None indicates 'no preference'
    cursor_icon: Option<egui::CursorIcon>,
}

impl Cursor {
    pub fn new() -> Self {
        Self {
            in_window: false,
            position: None, // because there's (currenlty) no way to know the initial cursor position until the first `WindowEvent::CursorMoved` event
            position_previous: None,
            position_frame_change: DVec2::ZERO,
            mouse_button_states: Default::default(),
            scroll_delta: DVec2::ZERO,
            cursor_icon: None,
        }
    }

    /// Update cursor position.
    /// * `position` - coords in pixels relative to the top-left corner of the window
    /// (equivilent to [`winit::event::CursorMoved::position`])
    pub fn set_position(&mut self, position: [f64; 2]) {
        self.position = Some(position.into());
        // if positions aren't initialized yet
        if self.position_previous.is_none() {
            self.position_previous = self.position;
        }
    }

    /// Update state of a mouse button
    /// * `winit_button` - winit mouse button enum
    /// * `state` - winit enum indicating event type (pressed or released)
    /// * `cursor_captured` - indicates if the gui wants exclusive use of this event
    pub fn set_click_state(
        &mut self,
        winit_button: winit::event::MouseButton,
        state: winit::event::ElementState,
        captured_by_gui: bool,
    ) {
        if captured_by_gui {
            return;
        }

        match MouseButton::from_winit(winit_button) {
            Ok(button) => {
                // button is only set to pressed when cursor hasn't been captured by e.g. gui
                self.mouse_button_states
                    .set(button, state, self.position.unwrap_or_default())
            }
            Err(e) => debug!("set_click_state: {}", e),
        };
    }

    /// Accumulate scroll travel values due to a winit scroll event. Use [`get_and_clear_scroll_delta`](Self::get_and_clear_scroll_delta)
    /// to query the total acccumulation.
    pub fn accumulate_scroll_delta(&mut self, delta: MouseScrollDelta, captured_by_gui: bool) {
        if !captured_by_gui {
            match delta {
                // can happen with mouse wheel or touchpad
                MouseScrollDelta::LineDelta(h, v) => {
                    self.scroll_delta += DVec2::new(h as f64, v as f64)
                }
                // happens if system supports it (whatever that means)
                MouseScrollDelta::PixelDelta(d) => self.scroll_delta += DVec2::new(d.x, d.y),
            }
        }
    }

    /// Update wherver the cursor is in the window area
    pub fn set_in_window_state(&mut self, is_in_window: bool) {
        self.in_window = is_in_window;
    }

    /// Process events to update internal state
    pub fn process_frame(&mut self) -> CursorEvent {
        // position processing
        self.position_frame_change =
            self.position.unwrap_or_default() - self.position_previous.unwrap_or_default();
        self.position_previous = self.position;

        // check previous states before ButtonStates::increment_frame() overwrites it
        let left_click_released_in_place = self.clicked_in_place(MouseButton::Left);
        let right_click_released_in_place = self.clicked_in_place(MouseButton::Right);
        let middle_click_released_in_place = self.clicked_in_place(MouseButton::Middle);

        self.mouse_button_states.increment_frame();

        if self.is_any_dragging() {
            self.cursor_icon = Some(egui::CursorIcon::Grabbing);
        } else {
            self.cursor_icon = None;
        }

        if left_click_released_in_place {
            CursorEvent::ClickInPlace(MouseButton::Left)
        } else if right_click_released_in_place {
            CursorEvent::ClickInPlace(MouseButton::Right)
        } else if middle_click_released_in_place {
            CursorEvent::ClickInPlace(MouseButton::Middle)
        } else {
            CursorEvent::None
        }
    }

    /// Will only be `None` before receiving the first [`WindowEvent::CursorMoved`](winit::event::WindowEvent::CursorMoved) event
    pub fn position(&self) -> Option<DVec2> {
        self.position
    }

    pub fn mouse_button_states(&self) -> MouseButtonStates {
        self.mouse_button_states
    }

    pub fn cursor_icon(&self) -> Option<egui::CursorIcon> {
        self.cursor_icon
    }

    /// Returns the change in cursor pixel position between the previous 2 [`Self::process_frame`] calls
    pub fn position_frame_change(&self) -> DVec2 {
        self.position_frame_change
    }

    /// Returns the accumulated horizontal and vertical scrolling since the last call to this function.
    /// Clears the internal scroll delta storage.
    pub fn get_and_clear_scroll_delta(&mut self) -> DVec2 {
        std::mem::take(&mut self.scroll_delta)
    }

    /// Returns true if the mouse button is down and the position has changed since it was first
    /// pressed down
    pub fn is_dragging(&self, mouse_button: MouseButton) -> bool {
        let mouse_button_state = self.mouse_button_states.get(mouse_button);
        if mouse_button_state.is_down() {
            let start_position = mouse_button_state
                .start_position()
                .expect("if `is_down` is true, can't be unheld");
            return start_position != self.position.unwrap_or_default();
        }
        false
    }
}

// ~~ Private Functions ~~

impl Cursor {
    fn is_any_dragging(&self) -> bool {
        for mouse_button in MOUSE_BUTTONS {
            if self.is_dragging(mouse_button) {
                return true;
            }
        }
        false
    }

    fn clicked_in_place(&self, mouse_button: MouseButton) -> bool {
        let mouse_button_state = self.mouse_button_states.get(mouse_button);
        mouse_button_state
            == ButtonState::JustReleased {
                // start position and current position are the same
                start_position: self.position.unwrap_or_default(),
            }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CursorEvent {
    None,
    ClickInPlace(MouseButton),
}
