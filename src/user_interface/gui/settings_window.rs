use super::Gui;
use crate::engine::settings::{Settings, SettingsIO};

impl Gui {
    pub(super) fn draw_settings_window(
        egui_context: &egui::Context,
        settings_window_visible: &mut bool,
        settings: &mut Settings,
        settings_io: &SettingsIO,
    ) {
        let add_contents = |ui: &mut egui::Ui| {
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
