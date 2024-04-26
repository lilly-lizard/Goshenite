use crate::user_interface::config_ui::DEFAULT_SCROLL_ZOOM_SENSITIVITY;

// ~~ Json Setting Names ~~

pub const SETTING_NAME_LOOK_MAPPING: &str = "cameraLookMapping";
pub const SETTING_NAME_LOOK_MAPPING_2: &str = "cameraLookMapping2";
pub const SETTING_NAME_PAN_MAPPING: &str = "cameraPanMapping";
pub const SETTING_NAME_PAN_MAPPING_2: &str = "cameraPanMapping2";
pub const SETTING_NAME_ZOOM_MAPPING: &str = "cameraZoomMapping";
pub const SETTING_NAME_ZOOM_MAPPING_2: &str = "cameraZoomMapping2";

pub const SETTING_NAME_MOUSE_BUTTON: &str = "mouseButton";
pub const SETTING_NAME_MODIFIERS: &str = "modifiers";

pub const SETTING_NAME_MOUSE_LEFT: &str = "left";
pub const SETTING_NAME_MOUSE_RIGHT: &str = "right";
pub const SETTING_NAME_MOUSE_MIDDLE: &str = "middle";
pub const SETTING_NAME_MOUSE_BACK: &str = "back";
pub const SETTING_NAME_MOUSE_FORWARD: &str = "forward";

pub const SETTING_NAME_SHIFT: &str = "shift";
pub const SETTING_NAME_CONTROL: &str = "control";
pub const SETTING_NAME_ALT: &str = "alt";

pub const SETTING_NAME_SCROLL_ZOOM_SENSITIVITY: &str = "scrollZoomSensitivity";

// ~~ Settings Struct ~~

#[derive(Clone, Copy)]
pub struct Settings {
    pub scroll_zoom_sensitivity: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            scroll_zoom_sensitivity: DEFAULT_SCROLL_ZOOM_SENSITIVITY,
        }
    }
}

// ~~ Json Encoding/Decoding Functions ~~
