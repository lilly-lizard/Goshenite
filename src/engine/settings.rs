use crate::user_interface::{
    camera::LookMode,
    config_ui::{CAMERA_DEFAULT_ARCBALL_TARGET_DEPTH, DEFAULT_SCROLL_ZOOM_SENSITIVITY},
    gui_state::DRAG_INC,
};

// ~~ Available Settings~~

pub struct CameraSettings {
    pub look_mode: LookMode,
    enabled_look_modes: Vec<LookMode>,
    /// Note: only used for `LookMode::ArcballHovering`
    pub arcball_target_depth: f64,
    pub scroll_zoom_sensitivity: f64,
    /// If true, camera enters arcball mode when an object or primitive op is selected
    pub arcball_on_select: bool,
}
pub struct RendererSettings {
    pub show_aabb_wireframe: bool,
}

// ~~ Defaults ~~

impl Default for CameraSettings {
    fn default() -> Self {
        CameraSettings {
            look_mode: LookMode::default(),
            enabled_look_modes: vec![LookMode::ArcballHovering, LookMode::PoV],
            arcball_target_depth: CAMERA_DEFAULT_ARCBALL_TARGET_DEPTH,
            scroll_zoom_sensitivity: DEFAULT_SCROLL_ZOOM_SENSITIVITY,
            arcball_on_select: false,
        }
    }
}
impl Default for RendererSettings {
    fn default() -> Self {
        RendererSettings {
            show_aabb_wireframe: false,
        }
    }
}

// ~~ Gui and JSON Definitions ~~

// General recomendations for writing `gui_fn`
// - bool => ui.checkbox(data, setting_name);
// - string => ui.text_edit_singleline(data);
// - float => ui.add(egui::DragValue::new(data).speed(DRAG_INC));
// - int => ui.add(egui::DragValue::new(data).speed(1));
impl Default for SettingsIO {
    fn default() -> Self {
        SettingsIO {
            categories: vec![SettingsCategory {
                name: "Camera".into(),
                settings: vec![SettingsIOEntry {
                    name: "Look Mode".into(),
                    description: "Available Modes:\n
- Arcball Hovering: an arcball that rotates around an invisible point in front of camera.\n
  Use setting `Arcball Target Depth` to control how far in front this point is.\n
- POV: turn around while camera remains in fixed position.\n
- Selected Object: arcball around the selected object.\n
- Selected Primitive Op: arcball around the selected primitive op.\n"
                        .into(),
                    gui_fn: |ui, settings, setting_name| {
                        let enabled_modes = settings.camera.enabled_look_modes().clone();
                        setting_ui_enum_some_disabled(ui, setting_name, &mut settings.camera.look_mode, &LookMode::VARIANTS, &enabled_modes);
                    },
                },
                SettingsIOEntry {
                    name: "Arcball Target Depth".into(),
                    description: "Distance from the camera to the arcball focus point when camera is in `Look Mode` == `Arcball Hovering`"
                        .into(),
                    gui_fn: |ui, settings, setting_name| {
                        let enabled = settings.camera.look_mode == LookMode::ArcballHovering;
                        ui.horizontal(|ui| {
                            ui.add_enabled(enabled, egui::DragValue::new(&mut settings.camera.arcball_target_depth).speed(DRAG_INC));
                            ui.add_enabled(enabled, egui::Label::new(setting_name));
                        });
                    },
                },
                SettingsIOEntry {
                    name: "Zoom Scroll Sensitivity".into(),
                    description: "Sensitivity when zooming via the scroll wheel."
                        .into(),
                    gui_fn: |ui, settings, setting_name| {
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut settings.camera.scroll_zoom_sensitivity).speed(DRAG_INC));
                            ui.label(setting_name);
                        });
                    },
                },
                SettingsIOEntry {
                    name: "Arcball on Select".into(),
                    description: "If enabled, camera enters arcball mode when an object or primitive op is selected."
                        .into(),
                    gui_fn: |ui, settings, setting_name| {
                        ui.checkbox(&mut settings.camera.arcball_on_select, setting_name);
                    },
                }],
            },
            SettingsCategory {
                name: "Camera".into(),
                settings: vec![
                    SettingsIOEntry {
                        name: "Show AABB Wireframe".into(),
                        description: "Render lines to show locations of axis aligned bounding boxes for every object."
                            .into(),
                        gui_fn: |ui, settings, setting_name| {
                            ui.checkbox(&mut settings.render.show_aabb_wireframe, setting_name);
                        },
                    }
                ]},
            ],
        }
    }
}

// ~~ Setting structs ~~

pub struct Settings {
    pub camera: CameraSettings,
    pub render: RendererSettings,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            camera: CameraSettings::default(),
            render: RendererSettings::default(),
        }
    }
}

pub struct SettingsIO {
    pub categories: Vec<SettingsCategory>,
}
pub struct SettingsCategory {
    pub name: String,
    pub settings: Vec<SettingsIOEntry>,
}
#[derive(Clone, PartialEq)]
#[allow(unpredictable_function_pointer_comparisons)]
pub struct SettingsIOEntry {
    pub name: String,
    pub description: String,
    pub gui_fn: fn(&mut egui::Ui, &mut Settings, &str),
}

impl SettingsIO {
    pub fn get_setting_entry_from_name(&self, setting_name: &str) -> Option<SettingsIOEntry> {
        for category in &self.categories {
            for setting in &category.settings {
                if &setting.name == setting_name {
                    return Some(setting.clone());
                }
            }
        }
        None
    }
}

impl CameraSettings {
    pub fn enabled_look_modes(&self) -> &Vec<LookMode> {
        &self.enabled_look_modes
    }

    pub fn unset_lock_on_target(&mut self) {
        if self.look_mode == LookMode::SelectedObject
            || self.look_mode == LookMode::SelectedPrimitiveOp
        {
            self.look_mode = LookMode::default();
        }
    }

    pub fn object_selected(&mut self) {
        if !self.enabled_look_modes.contains(&LookMode::SelectedObject) {
            self.enabled_look_modes.push(LookMode::SelectedObject);
        }
        if self.arcball_on_select {
            self.look_mode = LookMode::SelectedObject;
        }
    }

    pub fn primitive_op_selected(&mut self) {
        self.object_selected();
        if !self
            .enabled_look_modes
            .contains(&LookMode::SelectedPrimitiveOp)
        {
            self.enabled_look_modes.push(LookMode::SelectedPrimitiveOp);
        }
        if self.arcball_on_select {
            self.look_mode = LookMode::SelectedPrimitiveOp;
        }
    }

    pub fn object_deselected(&mut self) {
        self.primitive_op_deselected();
        if self.look_mode == LookMode::SelectedObject {
            self.look_mode = LookMode::default();
        }
        if let Some(index) = self
            .enabled_look_modes
            .iter()
            .position(|&x| x == LookMode::SelectedObject)
        {
            self.enabled_look_modes.remove(index);
        }
    }

    pub fn primitive_op_deselected(&mut self) {
        if self.look_mode == LookMode::SelectedPrimitiveOp {
            // assume object still selected
            self.look_mode = LookMode::SelectedObject;
        }
        if let Some(index) = self
            .enabled_look_modes
            .iter()
            .position(|&x| x == LookMode::SelectedPrimitiveOp)
        {
            self.enabled_look_modes.remove(index);
        }
    }
}

// ~~ UI Template Functions ~~

pub trait SettingEnum {
    fn value_display_name(&self) -> &str;
}

#[allow(unused)]
pub fn setting_ui_enum<T>(
    ui: &mut egui::Ui,
    setting_name: &str,
    setting_data: &mut T,
    variants: &[T],
) where
    T: SettingEnum + PartialEq + Clone,
{
    egui::ComboBox::from_label(setting_name)
        .selected_text(setting_data.value_display_name())
        .show_ui(ui, |ui| {
            for variant in variants {
                ui.selectable_value(setting_data, variant.clone(), variant.value_display_name());
            }
        });
}

pub fn setting_ui_enum_some_disabled<T>(
    ui: &mut egui::Ui,
    setting_name: &str,
    setting_data: &mut T,
    variants: &[T],
    enabled_variants: &[T],
) where
    T: SettingEnum + PartialEq + Clone,
{
    egui::ComboBox::from_label(setting_name)
        .selected_text(setting_data.value_display_name())
        .show_ui(ui, |ui| {
            for variant in variants {
                if enabled_variants.contains(variant) {
                    // selectable option
                    ui.selectable_value(
                        setting_data,
                        variant.clone(),
                        variant.value_display_name(),
                    );
                } else {
                    // disabled option
                    ui.add_enabled(false, egui::Button::new(variant.value_display_name()));
                }
            }
        });
}

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
