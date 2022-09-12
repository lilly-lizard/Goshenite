use glam::DVec2;
use log::debug;
use std::sync::Arc;
use winit::{
    event::ElementState,
    window::{CursorIcon, Window},
};

pub struct CursorState {
    window: Arc<Window>,
    /// Describes wherver the cursur is currently within the window bounds
    in_window: bool,
    /// The current cursor position
    position: DVec2,
    position_previous: DVec2,
    position_frame_change: DVec2,
    /// Describes wherver each mouse button is pressed
    is_pressed: ButtonStates,
    is_pressed_previous: ButtonStates,
    /// Which button (if any) is currently dragging (if multiple, set to the first)
    which_dragging: Option<MouseButton>,
}
impl CursorState {
    pub fn new(window: Arc<Window>) -> Self {
        Self {
            window,
            in_window: false,
            position: DVec2::default(),
            position_previous: DVec2::default(),
            position_frame_change: DVec2::default(),
            is_pressed: ButtonStates::default(),
            is_pressed_previous: ButtonStates::default(),
            which_dragging: None,
        }
    }

    pub fn set_new_position(&mut self, position: winit::dpi::PhysicalPosition<f64>) {
        let position: [f64; 2] = position.into();
        self.position = position.into();
    }

    pub fn set_click_state(
        &mut self,
        winit_button: winit::event::MouseButton,
        state: ElementState,
    ) {
        match MouseButton::from_winit(winit_button) {
            Ok(button) => self.is_pressed.set(button, state == ElementState::Pressed),
            Err(e) => debug!("{}", e),
        };
    }

    pub fn set_in_window_state(&mut self, is_in_window: bool) {
        self.in_window = is_in_window;
    }

    pub fn frame_update(&mut self) {
        // position processing
        self.position_frame_change = self.position - self.position_previous;
        let has_moved = self.position_frame_change.x != 0. && self.position_frame_change.y != 0.;
        self.position_previous = self.position;

        // button processing
        for button in MOUSE_BUTTONS {
            if let Some(dragging_button) = self.which_dragging {
                // if dragging set and button released, unset which_dragging
                if (dragging_button == button) && !self.is_pressed.get(button) {
                    self.which_dragging = None;
                }
            } else {
                // if button held and moving, set which_dragging
                if self.is_pressed.get(button) && self.is_pressed_previous.get(button) && has_moved
                {
                    self.which_dragging = Some(button);
                }
            }
            // update 'previous' records
            self.is_pressed_previous
                .set(button, self.is_pressed.get(button));
        }

        // set cursor icon
        let mut cursor_icon = CursorIcon::Default;
        if self.which_dragging.is_some() {
            cursor_icon = CursorIcon::Grabbing;
        }
        self.window.set_cursor_icon(cursor_icon);
    }

    pub fn position_frame_change(&self) -> DVec2 {
        self.position_frame_change
    }
    pub fn which_dragging(&self) -> Option<MouseButton> {
        self.which_dragging
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

/// Boolean value for each supported mouse button
#[derive(Default, Clone, Copy)]
struct ButtonStates {
    pub left: bool,
    pub right: bool,
    pub middle: bool,
    //button_4: ElementState,
    //button_5: ElementState,
}
impl ButtonStates {
    fn set(&mut self, button: MouseButton, state: bool) {
        match button {
            MouseButton::Left => self.left = state,
            MouseButton::Right => self.right = state,
            MouseButton::Middle => self.middle = state,
        }
    }
    fn get(&self, button: MouseButton) -> bool {
        match button {
            MouseButton::Left => self.left,
            MouseButton::Right => self.right,
            MouseButton::Middle => self.middle,
        }
    }
}
