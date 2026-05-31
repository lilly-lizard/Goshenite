use crate::{
    engine::settings::{Settings, SettingsIOEntry},
    user_interface::{
        config_ui::MAX_QUICK_ACCESS_SETTINGS,
        gui::{command_palette::GuiStateCommandPalette, Gui},
        view_modes::ViewMode,
    },
};
use egui_material_icons::icons::{
    ICON_KEYBOARD_COMMAND_KEY, ICON_LEFT_PANEL_CLOSE, ICON_LEFT_PANEL_OPEN, ICON_SETTINGS,
};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

impl Gui {
    /// Returns bar height
    pub(super) fn draw_bottom_bar(
        ui: &mut egui::Ui,
        side_panel_visible: &mut bool,
        settings_window_visible: &mut bool,
        view_mode: &mut ViewMode,
        command_pallette: &mut Option<GuiStateCommandPalette>,
        settings: &mut Settings,
        quick_access_settings: &mut [Option<SettingsIOEntry>; MAX_QUICK_ACCESS_SETTINGS],
    ) -> f32 {
        egui::Panel::bottom("bottom bar")
            .show_inside(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    bottom_bar_layout(
                        ui,
                        side_panel_visible,
                        settings_window_visible,
                        view_mode,
                        command_pallette,
                        settings,
                        quick_access_settings,
                    );
                });
            })
            .response
            .rect
            .height()
    }
}

fn bottom_bar_layout(
    ui: &mut egui::Ui,
    side_panel_visible: &mut bool,
    settings_window_visible: &mut bool,
    view_mode: &mut ViewMode,
    command_pallette: &mut Option<GuiStateCommandPalette>,
    settings: &mut Settings,
    quick_access_settings: &mut [Option<SettingsIOEntry>; MAX_QUICK_ACCESS_SETTINGS],
) {
    let mut command_pallette_visible = command_pallette.is_some();
    let mut settings_modified = false;

    let panel_icon = match side_panel_visible {
        true => ICON_LEFT_PANEL_CLOSE,
        false => ICON_LEFT_PANEL_OPEN,
    };
    ui.toggle_value(side_panel_visible, panel_icon)
        .on_hover_text("Toggle panel");

    egui::ComboBox::from_id_salt("")
        .selected_text(view_mode.name())
        .show_ui(ui, |ui| {
            for variant in ViewMode::VARIANTS {
                ui.selectable_value(view_mode, variant.clone(), variant.name());
            }
        });

    ui.separator();

    // quick access settings
    for maybe_setting in quick_access_settings {
        if let Some(setting) = maybe_setting {
            settings_modified =
                settings_modified || (setting.gui_fn)(ui, settings, &setting.name).changed();
        }
    }

    // right hand side (ui order right to left)
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        egui::widgets::global_theme_preference_switch(ui); // light/dark theme toggle
        egui::warn_if_debug_build(ui);

        ui.separator();

        ui.toggle_value(settings_window_visible, ICON_SETTINGS)
            .on_hover_text("Settings");
        if ui
            .toggle_value(&mut command_pallette_visible, ICON_KEYBOARD_COMMAND_KEY)
            .on_hover_text("Command Pallete")
            .changed()
        {
            *command_pallette = match command_pallette_visible {
                true => Some(Default::default()),
                false => None,
            };
        }
    });

    if settings_modified {
        settings.save_user_settings_json_file_async();
    }
}
