use crate::{
    engine::settings::{Settings, SettingsIOEntry},
    user_interface::{
        config_ui::MAX_QUICK_ACCESS_SETTINGS,
        gui::{command_palette::GuiStateCommandPalette, side_panel::SidePanelMode, Gui},
    },
};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

impl Gui {
    pub(super) fn draw_bottom_bar(
        ui: &mut egui::Ui,
        side_panel_mode: &mut Option<SidePanelMode>,
        settings_window_visible: &mut bool,
        command_pallette: &mut Option<GuiStateCommandPalette>,
        settings: &mut Settings,
        quick_access_settings: &mut [Option<SettingsIOEntry>; MAX_QUICK_ACCESS_SETTINGS],
    ) {
        egui::Panel::bottom("bottom bar").show_inside(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                bottom_bar_layout(
                    ui,
                    side_panel_mode,
                    settings_window_visible,
                    command_pallette,
                    settings,
                    quick_access_settings,
                );
            });
        });
    }
}

fn bottom_bar_layout(
    ui: &mut egui::Ui,
    side_panel_mode: &mut Option<SidePanelMode>,
    settings_window_visible: &mut bool,
    command_pallette: &mut Option<GuiStateCommandPalette>,
    settings: &mut Settings,
    quick_access_settings: &mut [Option<SettingsIOEntry>; MAX_QUICK_ACCESS_SETTINGS],
) {
    let mut command_pallette_visible = command_pallette.is_some();
    let (mut scene_visible, mut object_editor_visible) = SidePanelMode::bools(*side_panel_mode);
    let mut settings_modified = false;

    if ui.toggle_value(&mut scene_visible, "Scene").changed() {
        *side_panel_mode = match scene_visible {
            true => Some(SidePanelMode::Scene),
            false => None,
        };
    };
    if ui
        .toggle_value(&mut object_editor_visible, "Object Editor")
        .changed()
    {
        *side_panel_mode = match object_editor_visible {
            true => Some(SidePanelMode::ObjectEditor),
            false => None,
        };
    };

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

        ui.toggle_value(settings_window_visible, "Settings");
        if ui
            .toggle_value(&mut command_pallette_visible, "Command Pallete")
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
