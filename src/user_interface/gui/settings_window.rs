use super::Gui;
use crate::{engine::commands::Command, renderer::config_renderer::RenderDebugOptions};
use egui::Ui;

impl Gui {
    pub(super) fn draw_settings_window(
        egui_context: &egui::Context,
        settings_window_visible: &mut bool,
        render_debug_options: RenderDebugOptions,
    ) -> Vec<Command> {
        let mut commands = Vec::<Command>::new();

        let add_contents = |ui: &mut egui::Ui| {
            commands = layout_debug_options(ui, render_debug_options);
        };
        egui::Window::new("Settings")
            .open(settings_window_visible)
            .resizable(true)
            .vscroll(true)
            .hscroll(true)
            .show(egui_context, add_contents);

        commands
    }
}

fn layout_debug_options(ui: &mut Ui, old_render_options: RenderDebugOptions) -> Vec<Command> {
    let mut commands = Vec::<Command>::new();
    let mut new_render_options = old_render_options;

    // enable bounding box overlay
    let enable_aabb_wire_display = old_render_options.enable_aabb_wire_display;
    let aabb_button_res = ui.selectable_label(
        enable_aabb_wire_display,
        "Draw bounding boxes with wire-frame",
    );
    if aabb_button_res.clicked() {
        new_render_options.enable_aabb_wire_display = !new_render_options.enable_aabb_wire_display;
    }

    if new_render_options != old_render_options {
        commands.push(Command::SetRenderDebugOptions(new_render_options));
    }
    commands
}

// fn layout_camera_settings(ui: &mut Ui, camera: &Camera) -> Vec<Command> {
//     let mut new_mode = camera.look_mode();
//     egui::ComboBox::from_label("Camera mode")
//         .selected_text(camera.look_mode().display_name())
//         .show_ui(ui, |ui| {
//             ui.selectable_value(&mut new_mode, LookMode::ArcballHovering { direction })
//         });
//
// let mut new_arball_depth = camera.arcball_target_depth();
// ui.add(DragValue::new(&mut new_arball_depth).speed(DRAG_INC));
// if new_arball_depth != camera.arcball_target_depth() {

// }
// }
