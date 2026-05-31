use super::Gui;
use crate::{
    engine::settings::{Settings, SettingsIO, SettingsIOEntry},
    user_interface::config_ui::MAX_QUICK_ACCESS_SETTINGS,
};
use egui_material_icons::icons::ICON_SETTINGS;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

impl Gui {
    pub(super) fn draw_settings_window(
        egui_context: &egui::Context,
        settings_window_visible: &mut bool,
        settings: &mut Settings,
        settings_io: &SettingsIO,
        quick_access_settings: &mut [Option<SettingsIOEntry>; MAX_QUICK_ACCESS_SETTINGS],
    ) {
        let mut modified = false;
        let add_contents = |ui: &mut egui::Ui| {
            layout_settings_window(
                ui,
                &mut modified,
                settings,
                settings_io,
                quick_access_settings,
            );
        };
        egui::Window::new(ICON_SETTINGS.codepoint.to_string() + " Settings")
            .open(settings_window_visible)
            .resizable(true)
            .vscroll(true)
            .hscroll(true)
            .show(egui_context, add_contents);

        if modified {
            settings.save_user_settings_json_file_async();
        }
    }
}

fn layout_settings_window(
    ui: &mut egui::Ui,
    modified: &mut bool,
    settings: &mut Settings,
    settings_io: &SettingsIO,
    quick_access_settings: &mut [Option<SettingsIOEntry>; MAX_QUICK_ACCESS_SETTINGS],
) {
    ui.label("Quick Access Settings");
    for i in 0..MAX_QUICK_ACCESS_SETTINGS {
        let quick_access_setting = &mut quick_access_settings[i];
        let selected_text = match quick_access_setting {
            Some(setting) => &setting.name,
            None => "None",
        };
        let res = egui::ComboBox::from_label(i.to_string())
            .selected_text(selected_text)
            .show_ui(ui, |ui| {
                ui.selectable_value(quick_access_setting, None, "None");
                for category in &settings_io.categories {
                    for available_setting in &category.settings {
                        ui.selectable_value(
                            quick_access_setting,
                            Some(available_setting.clone()),
                            &available_setting.name,
                        );
                    }
                }
            });
        *modified = *modified || res.response.changed();
    }
    ui.separator();

    for category in &settings_io.categories {
        ui.label(category.name.clone());
        for setting in &category.settings {
            *modified = *modified
                || (setting.gui_fn)(ui, settings, &setting.name)
                    .on_hover_text(setting.description.clone())
                    .changed();
        }
        ui.separator();
    }

    if ui.button("Reset to defaults").clicked() {
        *settings = Settings::default();
        *modified = true;
    }
}
