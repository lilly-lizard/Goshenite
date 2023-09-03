use super::config_ui;
use glam::DVec2;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use winit::event::{ElementState, MouseScrollDelta};

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
    button_states: ButtonStates,
    /// Describes the state of the mouse buttons in the previous frame. Used to determine [`Cursor::which_dragging`]
    button_states_previous: ButtonStates,
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
            button_states_previous: Default::default(),
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
        state: ElementState,
        captured_by_gui: bool,
    ) {
        if captured_by_gui {
            return;
        }

        match MouseButton::from_winit(winit_button) {
            Ok(button) => {
                self.button_states
                    // button is only set to pressed when cursor hasn't been captured by e.g. gui
                    .set(button, state)
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
    pub fn process_frame(&mut self) {
        // position processing
        self.position_frame_change =
            self.position.unwrap_or_default() - self.position_previous.unwrap_or_default();
        let has_moved =
            self.position_frame_change.x != 0_f64 || self.position_frame_change.y != 0_f64;
        self.position_previous = self.position;

        // dragging logic
        if let Some(dragging_button) = self.which_dragging {
            // if which_dragging set but button released, unset which_dragging
            if self.button_states.is_released(dragging_button) {
                self.which_dragging = None;
                self.cursor_icon = None;
            }
        } else {
            // check each button
            for button in MOUSE_BUTTONS {
                // if button held and cursor has moved, set which_dragging
                if self.button_states.is_pressed(button)
                    && self.button_states_previous.is_pressed(button)
                {
                    if has_moved {
                        self.which_dragging = Some(button);
                        self.cursor_icon = Some(egui::CursorIcon::Grabbing);
                        break; // priority given to the order of `MOUSE_BUTTONS`
                    }
                }
            }
        }
        // update previous pressed state
        self.button_states_previous = self.button_states;
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

/// Mouse buttons supported by engine
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    //Button4,
    //Button5,
}

/// List of available [`MouseButton`] enum variations. Note that the order affects the priority for things like dragging logic.
static MOUSE_BUTTONS: [MouseButton; 3] =
    [MouseButton::Left, MouseButton::Right, MouseButton::Middle];
impl MouseButton {
    pub fn from_winit(button: winit::event::MouseButton) -> Result<Self, String> {
        match button {
            winit::event::MouseButton::Left => Ok(Self::Left),
            winit::event::MouseButton::Right => Ok(Self::Right),
            winit::event::MouseButton::Middle => Ok(Self::Middle),
            winit::event::MouseButton::Other(code) => match code {
                // todo check what actual button4/5 numbers turn up here
                //4 => Ok(&self.button_4),
                //5 => Ok(&self.button_5),
                _ => Err(format!(
                    "attempted to index unsupported mouse button code: {}",
                    code
                )),
            },
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    Released,
    Pressed,
    Held,
}

impl Default for ButtonState {
    fn default() -> Self {
        Self::Released
    }
}

#[derive(Default, Clone, Copy)]
struct MouseButtonStates {
    left: ButtonState,
    right: ButtonState,
    middle: ButtonState,

    previous_left: ButtonState,
    previous_right: ButtonState,
    previous_middle: ButtonState,
}

impl ButtonStates {
    fn set(&mut self, button: MouseButton, state: ElementState) {
        match button {
            MouseButton::Left => self.left = state,
            MouseButton::Right => self.right = state,
            MouseButton::Middle => self.middle = state,
        }
    }

    fn incriment_drag(&mut self) {
        todo!("drag logic");
    }

    fn get(&self, button: MouseButton) -> ElementState {
        match button {
            MouseButton::Left => self.left,
            MouseButton::Right => self.right,
            MouseButton::Middle => self.middle,
        }
    }

    fn is_pressed(&self, button: MouseButton) -> bool {
        let state = self.get(button);
        state == ElementState::Pressed
    }

    fn is_released(&self, button: MouseButton) -> bool {
        let state = self.get(button);
        state == ElementState::Released
    }

    fn is_held(&self, button: MouseButton) -> bool {
        let state = self.get(button);
        state == ButtonState::Held
    }
}
