use super::{
    button_state::MouseButtonStates,
    cursor::Cursor,
    keyboard_modifiers::{KeyboardModifier, KeyboardModifierStates},
    mouse_button::MouseButton,
};
use crate::engine::settings::{
    SETTING_NAME_LOOK_MAPPING, SETTING_NAME_LOOK_MAPPING_2, SETTING_NAME_MODIFIERS,
    SETTING_NAME_MOUSE_BUTTON, SETTING_NAME_PAN_MAPPING, SETTING_NAME_PAN_MAPPING_2,
    SETTING_NAME_ZOOM_MAPPING, SETTING_NAME_ZOOM_MAPPING_2,
};

// ~~ Camera Control Mouse Mapping ~~

pub const MAX_MODIFIERS: usize = 3;

/// Defines a combination of keyboard modifiers and a mouse button to for controls that require
/// mouse movement e.g. camera control.
#[derive(Default, Clone, Copy)]
pub struct MouseMapping {
    pub mouse_button: MouseButton,
    pub modifiers: [Option<KeyboardModifier>; MAX_MODIFIERS],
}

impl MouseMapping {
    pub fn mapping_active(
        &self,
        button_states: MouseButtonStates,
        modifier_states: KeyboardModifierStates,
    ) -> bool {
        if button_states.get(self.mouse_button).is_up() {
            return false;
        }
        for modifier in self.modifiers {
            if let Some(some_modifier) = modifier {
                if !modifier_states.is_pressed(some_modifier) {
                    return false;
                }
            }
        }
        true
    }
}

#[derive(Clone, Copy)]
pub struct CameraControlMappings {
    pub look: MouseMapping,
    pub look_2: Option<MouseMapping>,
    pub pan: MouseMapping,
    pub pan_2: Option<MouseMapping>,
    pub zoom: MouseMapping,
    pub zoom_2: Option<MouseMapping>,
}

impl CameraControlMappings {
    pub fn mappings_active_and_dragging_look(
        &self,
        cursor: &Cursor,
        modifier_states: KeyboardModifierStates,
    ) -> bool {
        mapping_active_and_dragging_general(cursor, modifier_states, self.look, self.look_2)
    }

    pub fn mappings_active_and_dragging_pan(
        &self,
        cursor: &Cursor,
        modifier_states: KeyboardModifierStates,
    ) -> bool {
        mapping_active_and_dragging_general(cursor, modifier_states, self.pan, self.pan_2)
    }

    pub fn mappings_active_and_dragging_zoom(
        &self,
        cursor: &Cursor,
        modifier_states: KeyboardModifierStates,
    ) -> bool {
        mapping_active_and_dragging_general(cursor, modifier_states, self.zoom, self.zoom_2)
    }
}

fn mapping_active_and_dragging_general(
    cursor: &Cursor,
    modifier_states: KeyboardModifierStates,
    mapping_1: MouseMapping,
    mapping_2: Option<MouseMapping>,
) -> bool {
    if mapping_1.mapping_active(cursor.mouse_button_states(), modifier_states)
        && cursor.is_dragging(mapping_1.mouse_button)
    {
        return true;
    }
    if let Some(some_mapping_2) = mapping_2 {
        return some_mapping_2.mapping_active(cursor.mouse_button_states(), modifier_states)
            && cursor.is_dragging(some_mapping_2.mouse_button);
    }
    false
}

impl Default for CameraControlMappings {
    fn default() -> Self {
        Self {
            look: MouseMapping {
                mouse_button: MouseButton::Left,
                ..Default::default()
            },
            look_2: None,
            pan: MouseMapping {
                mouse_button: MouseButton::Right,
                ..Default::default()
            },
            pan_2: None,
            zoom: MouseMapping {
                mouse_button: MouseButton::Middle,
                ..Default::default()
            },
            zoom_2: None,
        }
    }
}

// ~~ Json Setting Interpreter ~~

type JsonSettings = serde_json::Map<String, serde_json::Value>;

/// Consumes any settings used in `json_settings`
fn update_camera_control_mappings_from_json_settings(
    camera_control_mappings: &mut CameraControlMappings,
    json_settings: &mut JsonSettings,
) {
    // look camera mapping
    if let Some(mouse_mapping) =
        parse_mouse_mapping_setting(json_settings, &SETTING_NAME_LOOK_MAPPING)
    {
        camera_control_mappings.look = mouse_mapping;
    }
    if let Some(mouse_mapping) =
        parse_mouse_mapping_setting(json_settings, &SETTING_NAME_LOOK_MAPPING_2)
    {
        camera_control_mappings.look_2 = Some(mouse_mapping);
    }

    // pan camera mapping
    if let Some(mouse_mapping) =
        parse_mouse_mapping_setting(json_settings, &SETTING_NAME_PAN_MAPPING)
    {
        camera_control_mappings.pan = mouse_mapping;
    }
    if let Some(mouse_mapping) =
        parse_mouse_mapping_setting(json_settings, &SETTING_NAME_PAN_MAPPING_2)
    {
        camera_control_mappings.pan_2 = Some(mouse_mapping);
    }

    // zoom camera mapping
    if let Some(mouse_mapping) =
        parse_mouse_mapping_setting(json_settings, &SETTING_NAME_ZOOM_MAPPING)
    {
        camera_control_mappings.zoom = mouse_mapping;
    }
    if let Some(mouse_mapping) =
        parse_mouse_mapping_setting(json_settings, &SETTING_NAME_ZOOM_MAPPING_2)
    {
        camera_control_mappings.zoom_2 = Some(mouse_mapping);
    }
}

fn parse_mouse_mapping_setting(
    json_settings: &mut JsonSettings,
    mapping_setting_name: &'static str,
) -> Option<MouseMapping> {
    if let Some(possible_camera_look_setting) = json_settings.remove(mapping_setting_name) {
        if let serde_json::Value::Object(camera_json_settings) = possible_camera_look_setting {
            return get_mouse_mapping_from_mapping_settings(
                camera_json_settings,
                mapping_setting_name,
            );
        }
        println!(
            "invalid format for camera control setting: {}",
            mapping_setting_name
        );
    }
    None
}

fn get_mouse_mapping_from_mapping_settings(
    mut mapping_settings: JsonSettings,
    camera_json_setting_name: &'static str,
) -> Option<MouseMapping> {
    let mut mouse_mapping = MouseMapping::default();

    // todo test each warning
    println!(
        "todo mention {} in all warnings...",
        camera_json_setting_name
    );

    // mouse button
    if let Some(mouse_button) = get_mouse_button_from_mapping_settings(&mut mapping_settings) {
        mouse_mapping.mouse_button = mouse_button;
    } else {
        println!(
            "error: a mouse mapping must include a {} value",
            SETTING_NAME_MOUSE_BUTTON
        );
        return None;
    }

    // modifiers
    if let Some(possible_modifiers_setting) = mapping_settings.remove(SETTING_NAME_MODIFIERS) {
        if let serde_json::Value::Array(modifiers_array) = possible_modifiers_setting {
            set_mouse_mapping_modifiers_from_mapping_settings(
                modifiers_array,
                &mut mouse_mapping,
                camera_json_setting_name,
            );
        } else {
            println!("invalid format for {} setting", SETTING_NAME_MODIFIERS);
        }
    }

    // remaining json values are invalid
    for (json_string, _json_value) in mapping_settings {
        println!("invalid property: {}", json_string);
    }

    Some(mouse_mapping)
}

fn get_mouse_button_from_mapping_settings(
    mapping_settings: &mut serde_json::Map<String, serde_json::Value>,
) -> Option<MouseButton> {
    if let Some(possible_mouse_button_setting) = mapping_settings.remove(SETTING_NAME_MOUSE_BUTTON)
    {
        if let serde_json::Value::String(mouse_button_string) = possible_mouse_button_setting {
            if let Some(mouse_button) = MouseButton::from_setting_name(&mouse_button_string) {
                return Some(mouse_button);
            }

            println!(
                "invalid {} property: {}",
                SETTING_NAME_MOUSE_BUTTON, mouse_button_string
            );
        } else {
            println!("invalid format for {} setting", SETTING_NAME_MOUSE_BUTTON);
        }
    }
    None
}

fn set_mouse_mapping_modifiers_from_mapping_settings(
    modifiers_array: Vec<serde_json::Value>,
    mouse_mapping: &mut MouseMapping,
    camera_json_setting_name: &str,
) {
    let mut modifier_index: usize = 0;

    for modifier_setting in modifiers_array {
        if modifier_index >= MAX_MODIFIERS {
            println!(
                "there can only be maximum of {} unique modifiers per mouse mapping",
                MAX_MODIFIERS
            );
            return;
        }

        if let serde_json::Value::String(modifier_string) = modifier_setting {
            if let Some(modifier) = KeyboardModifier::from_setting_name(&modifier_string) {
                if mouse_mapping.modifiers.contains(&Some(modifier)) {
                    println!(
                        "duplicate modifier found for {} setting: {}",
                        camera_json_setting_name, modifier
                    );
                    continue;
                }

                // insert unique modifier
                mouse_mapping.modifiers[modifier_index] = Some(modifier);
                modifier_index = modifier_index + 1;
            } else {
                println!("invalid keyboard modifier name: {}", modifier_string);
            }
        } else {
            println!(
                "invalid property found in {} array: {}",
                SETTING_NAME_MODIFIERS, modifier_setting
            );
        }
    }
}
