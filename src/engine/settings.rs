use crate::{
    engine::engine_controller::EngineController,
    user_interface::{
        config_ui::DEFAULT_SCROLL_ZOOM_SENSITIVITY, controls_camera::CameraControlMappings,
        theme::Theme,
    },
};

// ~~ Json Setting Names ~~

pub const SETTING_NAME_LOOK_MAPPING: &str = "cameraLookMapping";
pub const SETTING_NAME_PAN_MAPPING: &str = "cameraPanMapping";
pub const SETTING_NAME_ZOOM_MAPPING: &str = "cameraZoomMapping";
pub const SETTING_NAME_ARCBALL_TARGET_MODIFIER: &str = "arcballTargetModifier";

pub const SETTING_NAME_MOUSE_BUTTON: &str = "mouseButton";
pub const SETTING_NAME_MODIFIERS: &str = "modifiers";
pub const SETTING_NAME_MODIFIER: &str = "modifier";

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

pub trait SettingDataType {
    fn process_update(&self, _engine: &mut EngineController) {}
    //fn from_string(self: &mut Self, string: String);
    fn display_name(&self) -> &str;
    fn from_string(&self, string: &str) -> Option<Box<dyn SettingDataType>>;
    fn ui(&mut self, ui: egui::Ui); // have helper functions for Enum, String, Bool and Number
}

pub struct Setting {
    pub name: String,
    pub data: Box<dyn SettingDataType>,
    pub updated: bool,
}

pub struct SettingCategory {
    pub name: String,
    pub settings: Vec<Setting>,
}

#[derive(Default)]
pub struct Settings {
    categories: Vec<SettingCategory>,
}
