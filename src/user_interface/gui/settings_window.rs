use super::Gui;
use crate::engine::settings::{setting_ui_primitive, Setting, SettingData, Settings};
use egui::Ui;

impl Gui {
    pub(super) fn draw_settings_window(
        egui_context: &egui::Context,
        settings_window_visible: &mut bool,
        settings: &mut Settings,
    ) {
        let add_contents = |ui: &mut egui::Ui| {
            for category in &mut settings.categories {
                ui.label(category.name.clone());
                for setting in &mut category.settings {
                    setting_ui(ui, setting);
                }
                ui.separator();
            }
        };
        egui::Window::new("Settings")
            .open(settings_window_visible)
            .resizable(true)
            .vscroll(true)
            .hscroll(true)
            .show(egui_context, add_contents);
    }
}

fn setting_ui(ui: &mut Ui, setting: &mut Setting) {
    match &mut setting.data {
        SettingData::DefinedType(data) => {
            data.ui(ui, &mut setting.updated);
        }
        SettingData::Primitive {
            setting_name,
            data,
            update_fn: _,
        } => {
            setting_ui_primitive(ui, setting_name, data, &mut setting.updated);
        }
    }
}
