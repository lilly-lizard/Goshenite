use super::Gui;
use crate::{engine::commands::Command, renderer::config_renderer::RenderOptions};
use egui::Ui;

impl Gui {
    pub(super) fn draw_debug_options_window(
        &mut self,
        render_options: RenderOptions,
    ) -> Vec<Command> {
        let mut commands = Vec::<Command>::new();

        let add_contents = |ui: &mut egui::Ui| {
            commands = layout_debug_options(ui, render_options);
        };
        egui::Window::new("Debug Options")
            .open(&mut self.sub_window_states.debug_options)
            .resizable(true)
            .vscroll(true)
            .hscroll(true)
            .show(&self.egui_context, add_contents);

        commands
    }
}

fn layout_debug_options(ui: &mut Ui, old_render_options: RenderOptions) -> Vec<Command> {
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
        commands.push(Command::SetRenderOptions(new_render_options));
    }
    commands
}
