use super::{
    button_state::MouseButtonStates,
    config_ui,
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
    button_states: MouseButtonStates,
    /// Which button (if any) is currently dragging (if multiple, set to the first)
    which_dragging: Option<MouseButton>,
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
            button_states: Default::default(),
            which_dragging: None,
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
                self.button_states.set(button, state)
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
                    self.scroll_delta +=
                        config_ui::SCROLL_SENSITIVITY * DVec2::new(h as f64, v as f64)
                }
                // happens if system supports it (whatever that means)
                MouseScrollDelta::PixelDelta(d) => {
                    self.scroll_delta += config_ui::SCROLL_SENSITIVITY * DVec2::new(d.x, d.y)
                }
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
        let has_moved =
            self.position_frame_change.x != 0_f64 || self.position_frame_change.y != 0_f64;
        self.position_previous = self.position;

        // check previous states before ButtonStates::increment_frame() overwrites it
        let left_click_released_in_place = self.button_states.get(MouseButton::Left).is_released()
            && self.button_states.get_previous(MouseButton::Left).is_down()
            && self.which_dragging.is_none();

        // dragging logic
        self.button_states.increment_frame();
        if let Some(dragging_button) = self.which_dragging {
            // if which_dragging set but button released, unset which_dragging
            if self.button_states.is_unheld(dragging_button) {
                self.which_dragging = None;
                self.cursor_icon = None;
            }
        } else {
            // check each button
            for button in MOUSE_BUTTONS {
                // if button held and cursor has moved, set which_dragging
                if self.button_states.is_held(button) {
                    if has_moved {
                        self.which_dragging = Some(button);
                        self.cursor_icon = Some(egui::CursorIcon::Grabbing);
                        // priority given to the order of `MOUSE_BUTTONS`
                        break;
                    }
                }
            }
        }

        if left_click_released_in_place {
            CursorEvent::ClickInPlace(MouseButton::Left)
        } else {
            CursorEvent::None
        }
    }

    /// Will only be `None` before receiving the first [`WindowEvent::CursorMoved`](winit::event::WindowEvent::CursorMoved) event
    pub fn position(&self) -> Option<DVec2> {
        self.position
    }

    pub fn get_cursor_icon(&self) -> Option<egui::CursorIcon> {
        self.cursor_icon
    }

    /// Returns the change in cursor pixel position between the previous 2 [`Self::process_frame`] calls
    pub fn position_frame_change(&self) -> DVec2 {
        self.position_frame_change
    }

    /// Returns which, if any, mouse button is in the dragging state
    pub fn which_dragging(&self) -> Option<MouseButton> {
        self.which_dragging
    }

    /// Returns the accumulated horizontal and vertical scrolling since the last call to this function.
    /// Clears the internal scroll delta storage.
    pub fn get_and_clear_scroll_delta(&mut self) -> DVec2 {
        std::mem::take(&mut self.scroll_delta)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CursorEvent {
    None,
    ClickInPlace(MouseButton),
}
