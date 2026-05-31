use self::command_palette::GuiStateCommandPalette;
use super::gui_state::ValueState;
use crate::{
    engine::{
        commands::Command,
        object::{
            object::ObjectId,
            object_collection::ObjectCollection,
            primitive_op::{PrimitiveOp, PrimitiveOpIndex},
        },
        save_states::{load_state_gui_positions, save_state_gui_positions},
        settings::{Settings, SettingsIO, SettingsIOEntry},
    },
    helper::more_errors::IoError,
    user_interface::{config_ui::MAX_QUICK_ACCESS_SETTINGS, view_modes::ViewMode},
};
use anyhow::Context;
use egui::{TextWrapMode, TexturesDelta};
use egui_winit::EventResponse;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::sync::Arc;
use winit::window::Window;

// various gui sections
mod bottom_bar;
mod command_palette;
mod object_editor;
mod scene_editor;
mod settings_window;
mod side_panel;

/// Controller for an [`egui`] immediate-mode gui
pub struct Gui {
    egui_context: egui::Context,
    window: Arc<Window>,
    winit_state: egui_winit::State,

    mesh_primitives: Vec<egui::ClippedPrimitive>,
    textures_delta_accumulation: Vec<TexturesDelta>,

    value_state: ValueState,
    side_panel_visible: bool,
    settings_window_visible: bool,
    command_pallette: Option<GuiStateCommandPalette>,
    quick_access_settings: [Option<SettingsIOEntry>; MAX_QUICK_ACCESS_SETTINGS],
}

// Public functions

impl Gui {
    /// Creates a new [`Gui`].
    /// * `window`: [`winit`] window
    /// * `max_texture_size`: maximum size of a texture. Corresponds to
    ///   VkPhysicalDeviceLimits.maxImageDimension2D
    pub fn new(
        window: Arc<Window>,
        settings_io: &SettingsIO,
        scale_factor: f32,
        max_texture_size: Option<usize>,
    ) -> Self {
        let egui_context = egui::Context::default();
        egui_context.set_global_style(egui::Style {
            // disable sentance wrap by default (horizontal scroll instead)
            wrap_mode: Some(TextWrapMode::Extend),
            ..Default::default()
        });

        let winit_state = egui_winit::State::new(
            egui_context.clone(),
            egui::ViewportId::ROOT,
            &window,
            Some(scale_factor),
            None,
            max_texture_size,
        );

        match load_state_gui_positions() {
            Ok(loaded_memory) => {
                egui_context.memory_mut(|context_memory| *context_memory = loaded_memory);
            }
            Err(e) => match e {
                IoError::FileDoesntExist(_file_path, _e) => info!("no gui window position state memory storage file found. initializing with default gui positions"),
                _ => ()
            }
        };

        egui_material_icons::initialize(&egui_context);

        let arcball_depth_setting = settings_io.get_setting_entry_from_name("Arcball Target Depth");
        let quick_access_settings = [arcball_depth_setting, None, None];

        Self {
            egui_context,
            window,
            winit_state,
            mesh_primitives: Default::default(),
            value_state: Default::default(),
            textures_delta_accumulation: Default::default(),
            side_panel_visible: true,
            settings_window_visible: false,
            command_pallette: None,
            quick_access_settings,
        }
    }

    /// Updates egui_context state by winit window event.
    /// Returns `true` if egui wants exclusive use of this event
    /// (e.g. a mouse click on an egui window, or entering text into a text field).
    /// For instance, if you use egui for a game, you want to first call this
    /// and only when this returns `false` pass on the events to your game.
    ///
    /// Note that egui uses `tab` to move focus between elements, so this will always return `true` for tabs.
    pub fn process_event(&mut self, event: &winit::event::WindowEvent) -> EventResponse {
        self.winit_state
            .on_window_event(self.window.as_ref(), event)
    }

    /// Get a reference to the clipped meshes required for rendering
    pub fn mesh_primitives(&self) -> &Vec<egui::ClippedPrimitive> {
        &self.mesh_primitives
    }

    #[allow(dead_code)]
    pub fn scale_factor(&self, window: &Window) -> f32 {
        egui_winit::pixels_per_point(&self.egui_context, window)
    }

    pub fn set_scale_factor(&mut self, scale_factor: f32) {
        self.egui_context.set_pixels_per_point(scale_factor);
    }

    /// Call this when a primitive op is selected
    pub fn update_selected_primitive_op(&mut self, selected_primitive_op: &PrimitiveOp) {
        self.value_state
            .set_selected_primitive_op_fields(selected_primitive_op);
    }

    pub fn update_gui(
        &mut self,
        settings: &mut Settings,
        settings_io: &SettingsIO,
        object_collection: &ObjectCollection,
        window: &Window,
        view_mode: &mut ViewMode,
        selected_object_id: Option<ObjectId>,
        selected_primitive_op_index: Option<PrimitiveOpIndex>,
    ) -> Vec<Command> {
        let mut commands = Vec::<Command>::new();

        let raw_input = self.winit_state.take_egui_input(window);

        let egui::FullOutput {
            platform_output,
            textures_delta,
            shapes,
            pixels_per_point,
            viewport_output: _,
        } = self.egui_context.run_ui(raw_input, |ui| {
            Self::draw_bottom_bar(
                ui,
                &mut self.side_panel_visible,
                &mut self.settings_window_visible,
                view_mode,
                &mut self.command_pallette,
                settings,
                &mut self.quick_access_settings,
            );

            if self.side_panel_visible {
                let mut new_commands = Self::draw_side_panel(
                    ui,
                    &mut self.value_state,
                    object_collection,
                    selected_object_id,
                    selected_primitive_op_index,
                );
                commands.append(&mut new_commands);
            }

            if self.settings_window_visible {
                Self::draw_settings_window(
                    &self.egui_context,
                    &mut self.settings_window_visible,
                    settings,
                    settings_io,
                    &mut self.quick_access_settings,
                );
            }

            if let Some(command_palette_state) = &mut self.command_pallette {
                let new_command =
                    Self::draw_command_palette(&self.egui_context, window, command_palette_state);
                if let Some(some_command) = new_command {
                    commands.push(some_command);
                    // close command palette after command has been selected
                    self.command_pallette = None;
                }
            }
        });

        self.winit_state
            .handle_platform_output(&self.window, platform_output);

        // store clipped primitive data for use by the renderer
        self.mesh_primitives = self.egui_context.tessellate(shapes, pixels_per_point);

        // store required texture changes for the renderer to apply updates
        if !textures_delta.is_empty() {
            self.textures_delta_accumulation.push(textures_delta);
        }

        commands
    }

    pub fn set_cursor_icon(&self, cursor_icon: egui::CursorIcon) {
        self.egui_context.set_cursor_icon(cursor_icon);
    }

    /// Returns texture update info accumulated since the last call to this function.
    pub fn get_and_clear_textures_delta(&mut self) -> Vec<TexturesDelta> {
        std::mem::take(&mut self.textures_delta_accumulation)
    }

    pub fn save_gui_state(&self) -> anyhow::Result<()> {
        self.egui_context
            .memory(save_state_gui_positions)
            .context("saving gui positions state")
    }

    pub fn toggle_command_palette_visability(&mut self) {
        self.command_pallette = match self.command_pallette {
            Some(_) => None,
            None => Some(Default::default()),
        }
    }

    pub fn hide_command_palette(&mut self) {
        self.command_pallette = None;
    }
}
