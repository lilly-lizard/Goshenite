use crate::{
    engine::engine::EngineControllers,
    user_interface::{
        camera::LookMode, config_ui::CAMERA_DEFAULT_ARCBALL_TARGET_DEPTH, gui_state::DRAG_INC,
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

// ~~ Default Settings ~~

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

        let settings_render = vec![Setting::new_primitive(
            "Enable AABB Wire Display",
            SettingPrimitive::Bool(false),
            None,
        )];

        Settings {
            categories: vec![
                SettingsCategory {
                    name: "Camera".to_string(),
                    settings: settings_camera,
                },
                SettingsCategory {
                    name: "Render".to_string(),
                    settings: settings_render,
                },
            ],
        }
    }
}

// ~~ Settings Structs ~~

pub struct Settings {
    pub categories: Vec<SettingsCategory>,
}
impl Settings {
    pub fn update_engine(&mut self, engine_controllers: &mut EngineControllers) {
        for category in &mut self.categories {
            for setting in &mut category.settings {
                if !setting.updated {
                    continue;
                }

                match &setting.data {
                    SettingData::DefinedType(data) => data.process_update(engine_controllers),
                    SettingData::Primitive {
                        data, update_fn, ..
                    } => {
                        if let Some(update_fn) = update_fn {
                            update_fn(data.clone(), engine_controllers);
                        }
                    }
                }
                setting.updated = false;
            }
        }
    }
    pub fn get_settings_render(&self) -> &SettingsCategory {
        for category in &self.categories {
            if category.name == "Render" {
                return category;
            }
        }
        panic!("Why is there no render settings category???");
    }
    pub fn get_settings_camera(&self) -> &SettingsCategory {
        for category in &self.categories {
            if category.name == "Camera" {
                return category;
            }
        }
        panic!("Why is there no camera settings category???");
    }
}

pub struct SettingsCategory {
    pub name: String,
    pub settings: Vec<Setting>,
}
impl SettingsCategory {
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
        update_fn: Option<fn(data: SettingPrimitive, engine_controllers: &mut EngineControllers)>,
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

#[derive(Clone, PartialEq)]
pub enum SettingPrimitive {
    Bool(bool),
    String(String),
    Float(f64),
    Int(i32),
}
pub enum SettingData {
    DefinedType(Box<dyn SettingDataType>),
    Primitive {
        setting_name: &'static str,
        data: SettingPrimitive,
        update_fn: Option<fn(data: SettingPrimitive, engine_controllers: &mut EngineControllers)>,
    },
}

pub trait SettingDataType {
    /// E.g. "Camera Look Mode"
    fn setting_name(&self) -> &str;
    /// E.g. "Arcball Hovering"
    fn value_display_name(&self) -> &str;
    /// Can use helper functions found below e.g. `setting_ui_enum()`
    fn ui(&mut self, ui: &mut egui::Ui, updated: &mut bool);
    /// Optionally implement if the engine's state needs to be changed when this setting is changed
    fn process_update(&self, _engine_controllers: &mut EngineControllers) {}
}

// ~~ UI Template Functions ~~

pub fn setting_ui_primitive(
    ui: &mut egui::Ui,
    setting_name: &'static str,
    setting_data: &mut SettingPrimitive,
    updated: &mut bool,
) {
    *updated |= match setting_data {
        SettingPrimitive::Bool(data) => ui.checkbox(data, setting_name),
        SettingPrimitive::String(data) => ui.text_edit_singleline(data),
        SettingPrimitive::Float(data) => ui.add(egui::DragValue::new(data).speed(DRAG_INC)),
        SettingPrimitive::Int(data) => ui.add(egui::DragValue::new(data).speed(1)),
    }
    .changed();
}

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
