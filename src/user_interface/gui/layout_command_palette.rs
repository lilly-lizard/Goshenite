use super::Gui;
use crate::engine::commands::Command;
use winit::window::Window;

impl Gui {
    pub(super) fn draw_command_palette(&mut self, window: &Window) -> Vec<Command> {
        let mut commands = Vec::<Command>::new();

        // pos: top of window
        // max width/height but cap it if window is too small
        // caps: max width, then 0.5 of window width
        // caps: half height, until minimum, then capped by bottom panel
        const DEFAULT_WIDTH: f32 = 60.;
        const MIN_HEIGHT: f32 = 10.;
        let window_size = window.inner_size();
        let width = f32::min(DEFAULT_WIDTH, 0.6 * window_size.width as f32);
        let height = f32::max(MIN_HEIGHT, 0.5 * window_size.height as f32);

        let add_contents = |ui: &mut egui::Ui| {
            commands = layout_command_palette(ui);
        };
        egui::Window::new("Command Palette")
            .open(&mut self.sub_window_states.command_palette)
            .resizable(false)
            .vscroll(true)
            .fixed_size([width, height])
            .show(&self.context, add_contents);

        commands
    }
}

pub fn layout_command_palette(ui: &mut egui::Ui) -> Vec<Command> {
    todo!()
}
