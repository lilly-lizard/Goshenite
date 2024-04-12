//! Also contains the list of commands available via the command palette.
use super::Gui;
use crate::{engine::commands::Command, helper::index_in_list::IndexInList};
use winit::window::Window;

// ~~ Available Commands ~~

/// Note: only make entries for commands that have no arguments/members because the user can only
/// select the command and cannot supply any arguments.
#[derive(Debug, Clone)]
struct CommandPaletteEntry {
    name: &'static str,
    command: Command,
}

const AVAILABLE_PALETTE_COMMANDS: [CommandPaletteEntry; 4] = [
    CommandPaletteEntry {
        name: "Save Camera State",
        command: Command::SaveStateCamera,
    },
    CommandPaletteEntry {
        name: "Load Camera State",
        command: Command::LoadStateCamera,
    },
    CommandPaletteEntry {
        name: "Save All Objects",
        command: Command::SaveAllObjects,
    },
    CommandPaletteEntry {
        name: "Load Objects",
        command: Command::LoadObjects,
    },
];

// ~~ Drawing fns ~~

impl Gui {
    pub(super) fn draw_command_palette(&mut self, window: &Window) -> Option<Command> {
        // pos: top of window
        // max width/height but cap it if window is too small
        // caps: max width, then 0.5 of window width
        // caps: half height, until minimum, then capped by bottom panel
        const DEFAULT_WIDTH: f32 = 60.;
        const MIN_HEIGHT: f32 = 10.;
        let window_size = window.inner_size();
        let width = f32::min(DEFAULT_WIDTH, 0.6 * window_size.width as f32);
        let height = f32::max(MIN_HEIGHT, 0.5 * window_size.height as f32);

        let mut new_command = None;
        let add_contents = |ui: &mut egui::Ui| {
            new_command = layout_command_palette(ui, &mut self.command_palette_state);
        };
        egui::Window::new("Command Palette")
            .open(&mut self.sub_window_states.command_palette)
            .resizable(false)
            .vscroll(true)
            .fixed_size([width, height])
            .show(&self.egui_context, add_contents);

        new_command
    }
}

#[allow(unused_parens)]
pub fn layout_command_palette(
    ui: &mut egui::Ui,
    gui_state: &mut GuiStateCommandPalette,
) -> Option<Command> {
    // command list
    for (command_index, palette_command) in AVAILABLE_PALETTE_COMMANDS.iter().enumerate() {
        let mut highlighted = false;
        if let Some(selected_index) = gui_state.highlighted_command_index.index() {
            highlighted = (selected_index == command_index);
        }

        let ui_res = ui.selectable_label(highlighted, palette_command.name);
        let selected = ui_res.clicked();
        if selected {
            gui_state.reset();
            let selected_command = AVAILABLE_PALETTE_COMMANDS[command_index].command.clone();
            return Some(selected_command);
        }
    }
    None
}

// ~~ Command Field Gui State ~~

#[derive(Debug, Clone, Default)]
pub struct GuiStateCommandPalette {
    pub highlighted_command_index: IndexInList,
    pub user_input_text: String,
}

impl GuiStateCommandPalette {
    pub fn reset(&mut self) {
        self.highlighted_command_index = Default::default();
        self.user_input_text = Default::default();
    }
}
