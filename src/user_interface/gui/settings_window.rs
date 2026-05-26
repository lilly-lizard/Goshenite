use super::Gui;
use crate::{
    engine::settings::{Settings, SettingsIO, SettingsIOEntry},
    user_interface::config_ui::MAX_QUICK_ACCESS_SETTINGS,
};

impl Gui {
    pub(super) fn draw_settings_window(
        egui_context: &egui::Context,
        settings_window_visible: &mut bool,
        settings: &mut Settings,
        settings_io: &SettingsIO,
        quick_access_settings: &mut [Option<SettingsIOEntry>; MAX_QUICK_ACCESS_SETTINGS],
    ) {
        let add_contents = |ui: &mut egui::Ui| {
            ui.label("Quick Access Settings");
            for i in 0..MAX_QUICK_ACCESS_SETTINGS {
                let quick_access_setting = &mut quick_access_settings[i];
                let selected_text = match quick_access_setting {
                    Some(setting) => &setting.name,
                    None => "None",
                };
                egui::ComboBox::from_label(i.to_string())
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
            }
            ui.separator();

            for category in &settings_io.categories {
                ui.label(category.name.clone());
                for setting in &category.settings {
                    (setting.gui_fn)(ui, settings, &setting.name)
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
