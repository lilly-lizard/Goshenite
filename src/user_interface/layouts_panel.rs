use super::config_ui::EGUI_TRACE;
use egui::Ui;

pub fn top_panel_layout(ui: &mut Ui) {
    if EGUI_TRACE {
        egui::trace!(ui);
    }
    ui.horizontal_wrapped(|ui| {
        ui.visuals_mut().button_frame = false; // idk what this does tbh

        // light/dark theme toggle
        egui::widgets::global_dark_light_mode_switch(ui);
    });
}
