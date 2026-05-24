use crate::{
    engine::engine_controller::EngineController,
    user_interface::{
        camera::LookMode,
        config_ui::{CAMERA_DEFAULT_ARCBALL_TARGET_DEPTH, DEFAULT_SCROLL_ZOOM_SENSITIVITY},
        controls_camera::CameraControlMappings,
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
    /// E.g. "Camera Look Mode"
    fn setting_name(&self) -> &str;
    /// E.g. "Arcball Hovering"
    fn value_display_name(&self) -> &str;
    /// Can use helper functions found below e.g. `setting_ui_enum()`
    fn ui(&mut self, ui: &mut egui::Ui, updated: &mut bool);
    /// Optionally implement if the engine's state needs to be changed when this setting is changed
    fn process_update(&self, _engine: &mut EngineController) {}
}

pub enum SettingPrimitive {
    String(String),
    Bool(bool),
    Float(f64),
    Int(i32),
}
pub enum SettingData {
    DefinedType(Box<dyn SettingDataType>),
    Primitive {
        setting_name: &'static str,
        data: SettingPrimitive,
        update_fn: Option<fn(data: SettingPrimitive, engine: &mut EngineController)>,
    },
}

pub struct Setting {
    pub data: SettingData,
    pub updated: bool,
}
impl Setting {
    pub fn new_type<T: Default + SettingDataType + 'static>() -> Self {
        Self {
            data: SettingData::DefinedType(Box::new(T::default())),
            updated: false,
        }
    }
    pub fn new_primitive(
        setting_name: &'static str,
        data: SettingPrimitive,
        update_fn: Option<fn(data: SettingPrimitive, engine: &mut EngineController)>,
    ) -> Self {
        Self {
            data: SettingData::Primitive {
                setting_name,
                data,
                update_fn,
            },
            updated: false,
        }
    }
}

pub struct SettingCategory {
    pub name: String,
    pub settings: Vec<Setting>,
}
impl SettingCategory {
    pub fn get_setting(&self, name: String) -> Option<&Setting> {
        for setting in &self.settings {
            match &setting.data {
                SettingData::DefinedType(data) => {
                    if data.setting_name() == name {
                        return Some(setting);
                    }
                }
                SettingData::Primitive { setting_name, .. } => {
                    if *setting_name == name {
                        return Some(setting);
                    }
                }
            }
        }
        None
    }
}

pub struct Settings {
    categories: Vec<SettingCategory>,
}

impl Default for Settings {
    fn default() -> Self {
        let settings_camera = vec![
            Setting::new_type::<LookMode>(),
            Setting::new_primitive(
                "Arcball Target Depth",
                SettingPrimitive::Float(CAMERA_DEFAULT_ARCBALL_TARGET_DEPTH),
                Some(|data, engine| {
                    if let SettingPrimitive::Float(depth) = data {
                        engine.camera.set_arcball_target_depth(depth);
                    }
                }),
            ),
        ];

        let settings_debug = vec![Setting::new_primitive(
            "Enable AABB Wire Display",
            SettingPrimitive::Bool(false),
            None,
        )];

        Settings {
            categories: vec![
                SettingCategory {
                    name: "Camera".to_string(),
                    settings: settings_camera,
                },
                SettingCategory {
                    name: "Debug".to_string(),
                    settings: settings_debug,
                },
            ],
        }
    }
}

// ~~ UI Template Functions ~~

pub fn setting_ui_enum<T>(
    ui: &mut egui::Ui,
    setting_data: &mut T,
    variants: &[T],
    updated: &mut bool,
) where
    T: SettingDataType + PartialEq + Clone,
{
    egui::ComboBox::from_label(setting_data.setting_name())
        .selected_text(setting_data.value_display_name())
        .show_ui(ui, |ui| {
            for variant in variants {
                *updated |= ui
                    .selectable_value(setting_data, variant.clone(), variant.value_display_name())
                    .changed();
            }
        });
}
