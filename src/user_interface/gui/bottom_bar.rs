use crate::{
    engine::settings::CameraSettings,
    user_interface::{
        camera::LookMode,
        gui::{command_palette::GuiStateCommandPalette, side_panel::SidePanelMode, Gui},
        gui_state::DRAG_INC,
    },
};
use egui::{DragValue, Ui};

impl Gui {
    pub(super) fn draw_bottom_bar(
        ui: &mut egui::Ui,
        side_panel_mode: &mut Option<SidePanelMode>,
        settings_window_visible: &mut bool,
        command_pallette: &mut Option<GuiStateCommandPalette>,
        camera_settings: &mut CameraSettings,
    ) {
        egui::Panel::bottom("bottom bar").show_inside(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                bottom_bar_layout(
                    ui,
                    side_panel_mode,
                    settings_window_visible,
                    command_pallette,
                    camera_settings,
                );
            });
        });
    }
}

fn bottom_bar_layout(
    ui: &mut Ui,
    side_panel_mode: &mut Option<SidePanelMode>,
    settings_window_visible: &mut bool,
    command_pallette: &mut Option<GuiStateCommandPalette>,
    camera_settings: &mut CameraSettings,
) {
    let mut command_pallette_visible = command_pallette.is_some();
    let (mut scene_visible, mut object_editor_visible) = SidePanelMode::bools(*side_panel_mode);

    // left hand side
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

    // right hand side (ui order right to left)
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        egui::widgets::global_theme_preference_switch(ui); // light/dark theme toggle
        egui::warn_if_debug_build(ui);

        if camera_settings.look_mode == LookMode::ArcballHovering {
            ui.add(DragValue::new(&mut camera_settings.arcball_target_depth).speed(DRAG_INC));
        }

        if ui
            .toggle_value(&mut command_pallette_visible, "Command Pallete")
            .changed()
        {
            *command_pallette = match command_pallette_visible {
                true => Some(Default::default()),
                false => None,
            };
        }
        ui.toggle_value(settings_window_visible, "Settings");
    });
}
