use crate::engine::{
    object::{object::ObjectRef, object_collection::ObjectCollection, objects_delta::ObjectsDelta},
    primitives::primitive_references::PrimitiveReferences,
};
use egui::{Button, FontFamily::Proportional, FontId, RichText, TextStyle, TexturesDelta};
use egui_winit::EventResponse;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::{rc::Weak, sync::Arc};
use winit::{event_loop::EventLoopWindowTarget, window::Window};

use super::{
    gui_layouts::{object_list, primitive_op_editor, primitive_op_list},
    gui_state::GuiState,
};

/// Controller for an [`egui`] immediate-mode gui
pub struct Gui {
    window: Arc<Window>,
    context: egui::Context,
    window_state: egui_winit::State,
    mesh_primitives: Vec<egui::ClippedPrimitive>,
    state: GuiState,
    textures_delta: Vec<TexturesDelta>,
    objects_delta: ObjectsDelta,
}

// Public functions

impl Gui {
    /// Creates a new [`Gui`].
    /// * `window`: [`winit`] window
    /// * `max_texture_side`: maximum size of a texture. Query from graphics driver using
    /// [`crate::renderer::render_manager::RenderManager::max_image_array_layers`]
    pub fn new<T>(
        event_loop: &EventLoopWindowTarget<T>,
        window: Arc<winit::window::Window>,
    ) -> Self {
        let context = egui::Context::default();
        context.set_style(egui::Style {
            // disable sentance wrap by default (horizontal scroll instead)
            wrap: Some(false),
            ..Default::default()
        });
        let mut window_state = egui_winit::State::new(event_loop);
        // set egui scale factor to platform dpi (by default)
        window_state.set_pixels_per_point(window.scale_factor() as f32);
        Self {
            window: window.clone(),
            context,
            window_state,
            mesh_primitives: Default::default(),
            state: Default::default(),
            textures_delta: Default::default(),
            objects_delta: Default::default(),
        }
    }

    /// Updates context state by winit window event.
    /// Returns `true` if egui wants exclusive use of this event
    /// (e.g. a mouse click on an egui window, or entering text into a text field).
    /// For instance, if you use egui for a game, you want to first call this
    /// and only when this returns `false` pass on the events to your game.
    ///
    /// Note that egui uses `tab` to move focus between elements, so this will always return `true` for tabs.
    pub fn process_event(&mut self, event: &winit::event::WindowEvent<'_>) -> EventResponse {
        self.window_state.on_event(&self.context, event)
    }

    /// Get a reference to the clipped meshes required for rendering
    pub fn mesh_primitives(&self) -> &Vec<egui::ClippedPrimitive> {
        &self.mesh_primitives
    }

    pub fn scale_factor(&self) -> f32 {
        self.window_state.pixels_per_point()
    }

    pub fn update_gui(
        &mut self,
        object_collection: &ObjectCollection,
        primitive_references: &PrimitiveReferences,
    ) -> anyhow::Result<()> {
        // begin frame
        let raw_input = self.window_state.take_egui_input(self.window.as_ref());
        self.context.begin_frame(raw_input);

        self.objects_window(object_collection);
        self.object_editor_window(primitive_references);

        // end frame
        let egui::FullOutput {
            platform_output,
            repaint_after: _r,
            textures_delta,
            shapes,
        } = self.context.end_frame();
        self.window_state.handle_platform_output(
            self.window.as_ref(),
            &self.context,
            platform_output,
        );

        // store clipped primitive data for use by the renderer
        self.mesh_primitives = self.context.tessellate(shapes);

        // store required texture changes for the renderer to apply updates
        if !textures_delta.is_empty() {
            self.textures_delta.push(textures_delta);
        }

        Ok(())
    }

    /// Returns texture update info accumulated since the last call to this function.
    pub fn get_and_clear_textures_delta(&mut self) -> Vec<TexturesDelta> {
        std::mem::take(&mut self.textures_delta)
    }

    /// Returns a description of the changes to objects since last call to this function.
    pub fn get_and_clear_objects_delta(&mut self) -> ObjectsDelta {
        std::mem::take(&mut self.objects_delta)
    }

    pub fn selected_object(&self) -> Option<Weak<ObjectRef>> {
        self.state.selected_object.clone()
    }
}

// Private functions

impl Gui {
    fn objects_window(&mut self, object_collection: &ObjectCollection) {
        // ui layout closure
        let add_contents = |ui: &mut egui::Ui| {
            object_list(ui, &mut self.state, object_collection);
        };

        // add window to egui context
        egui::Window::new("Objects")
            .resizable(true)
            .vscroll(true)
            .hscroll(true)
            .show(&self.context, add_contents);
    }

    fn object_editor_window(&mut self, primitive_references: &PrimitiveReferences) {
        // ui layout closure
        let add_contents = |ui: &mut egui::Ui| {
            let no_object_text = RichText::new("No Object Selected...").italics();
            let selected_object_weak = match &self.state.selected_object {
                Some(o) => o.clone(),
                None => {
                    ui.label(no_object_text);
                    return;
                }
            };
            let selected_object_ref = match selected_object_weak.upgrade() {
                Some(o) => o,
                None => {
                    debug!("selected object dropped. deselecting object...");
                    self.state.deselect_object();
                    ui.label(no_object_text);
                    return;
                }
            };

            ui.heading(format!("{}", selected_object_ref.borrow().name));
            primitive_op_editor(
                ui,
                &mut self.state,
                &mut self.objects_delta,
                &selected_object_ref.borrow(),
                primitive_references,
            );
            primitive_op_list(
                ui,
                &mut self.state,
                &mut self.objects_delta,
                &mut selected_object_ref.borrow_mut(),
            );
        };

        // add window to egui context
        egui::Window::new("Object Editor")
            .resizable(true)
            .vscroll(true)
            .hscroll(true)
            .show(&self.context, add_contents);
    }

    fn _bug_test_window(&mut self) {
        let add_contents = |ui: &mut egui::Ui| {
            // TODO TESTING tests GuiRenderer create_texture() functionality for when ImageDelta.pos != None
            // todo add to testing window function and document
            ui.separator();
            if ui.add(Button::new("gui bug test")).clicked() {
                let style = &*self.context.style();
                let mut style = style.clone();
                style.text_styles = [
                    (TextStyle::Heading, FontId::new(20.0, Proportional)),
                    (TextStyle::Body, FontId::new(18.0, Proportional)),
                    (TextStyle::Monospace, FontId::new(14.0, Proportional)),
                    (TextStyle::Button, FontId::new(14.0, Proportional)),
                    (TextStyle::Small, FontId::new(10.0, Proportional)),
                ]
                .into();
                self.context.set_style(style);
            }
        };
        egui::Window::new("Gui Bug Test")
            .resizable(true)
            .vscroll(true)
            .hscroll(true)
            .show(&self.context, add_contents);
    }
}
