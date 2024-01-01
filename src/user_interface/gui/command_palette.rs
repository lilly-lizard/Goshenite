use super::Gui;
use crate::{
    engine::{commands::Command, object::object::ObjectId},
    helper::{index_in_list::IndexInList, unique_id_gen::UniqueId},
};
use egui::{Key, TextEdit};
use winit::window::Window;

// ~~ Command Field Gui State ~~

#[derive(Debug, Clone, Default)]
pub struct GuiStateCommandPalette {
    pub highlighted_command_index: IndexInList,
    pub current_parameter_gui_fn:
        Option<fn(&mut egui::Ui, &mut GuiStateCommandPalette) -> Option<Command>>,

    // command parameter gui state
    pub user_input_text: String,
}

impl GuiStateCommandPalette {
    pub fn reset(&mut self) {
        self.highlighted_command_index = Default::default();
        self.current_parameter_gui_fn = None;
        self.user_input_text = Default::default();
    }
}

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
            .show(&self.context, add_contents);

        new_command
    }
}

pub fn layout_command_palette(
    ui: &mut egui::Ui,
    gui_state: &mut GuiStateCommandPalette,
) -> Option<Command> {
    // user enters parameters for previously selected command
    if let Some(parameter_gui_fn) = gui_state.current_parameter_gui_fn {
        let parameter_gui_fn_res = parameter_gui_fn(ui, gui_state);
        if parameter_gui_fn_res.is_some() {
            // command palette will be closed after returning Some(command)
            gui_state.reset();
        }
        return parameter_gui_fn_res;
    }

    // command list
    for (command_index, palette_command) in AVAILABLE_PALETTE_COMMANDS.iter().enumerate() {
        let highlighted = if let Some(selected_index) = gui_state.highlighted_command_index.index()
        {
            selected_index == command_index
        } else {
            false
        };
        let ui_res = ui.selectable_label(highlighted, palette_command.name);

        let selected = ui_res.clicked();
        if selected {
            match &palette_command.command_source {
                CommandPaletteSource::SingleCommand(selected_command) => {
                    // command palette will be closed after returning Some(command)
                    gui_state.reset();
                    return Some(selected_command.clone());
                }
                CommandPaletteSource::ParameterGuiFn(paramter_gui_fn) => {
                    // can't return command just yet, user will have to enter parameters next frame
                    gui_state.current_parameter_gui_fn = Some(*paramter_gui_fn)
                }
            }
        }
    }
    None
}

// ~~ Available Commands ~~

#[derive(Debug, Clone)]
struct CommandPaletteEntry {
    name: &'static str,
    command_source: CommandPaletteSource,
}

/// Describes how to get the final command when a command palette entry is selected
#[derive(Debug, Clone)]
enum CommandPaletteSource {
    /// When this command entry is selected, this command is returned.
    SingleCommand(Command),
    /// When this command entry is selected, a gui is brought up where the user has to enter
    /// additional parameters.
    ParameterGuiFn(fn(&mut egui::Ui, &mut GuiStateCommandPalette) -> Option<Command>),
}

const AVAILABLE_PALETTE_COMMANDS: [CommandPaletteEntry; 3] = [
    CommandPaletteEntry {
        name: "Save Camera State",
        command_source: CommandPaletteSource::SingleCommand(Command::SaveStateCamera),
    },
    CommandPaletteEntry {
        name: "Load Camera State",
        command_source: CommandPaletteSource::SingleCommand(Command::LoadStateCamera),
    },
    CommandPaletteEntry {
        name: "Select Object",
        command_source: CommandPaletteSource::ParameterGuiFn(parameters_gui_select_object),
    },
];

// ~~ Command Field Gui Functions ~~

fn parameters_gui_select_object(
    ui: &mut egui::Ui,
    gui_state: &mut GuiStateCommandPalette,
) -> Option<Command> {
    let user_input_widget =
        TextEdit::singleline(&mut gui_state.user_input_text).hint_text("Object ID");
    let response = ui.add(user_input_widget);

    // if enter key pressed while the text field was focused
    if response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
        let possible_id = gui_state.user_input_text.parse::<UniqueId>();
        let Ok(raw_id) = possible_id else {
            // todo error message or just make textbox red
            return None;
        };
        let object_id: ObjectId = raw_id.into();
        return Some(Command::SelectObject(object_id));
    }
    return None;
}
