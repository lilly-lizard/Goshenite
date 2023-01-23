use crate::engine::{
    object::{
        object::{Object, ObjectRef},
        object_collection::ObjectCollection,
    },
    primitives::{primitive::Primitive, primitive_references::PrimitiveReferences},
};
use egui::{
    Button, Checkbox, ComboBox, DragValue, FontFamily::Proportional, FontId, Sense, TexturesDelta,
};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::{
    rc::{Rc, Weak},
    sync::Arc,
};
use winit::{event_loop::EventLoopWindowTarget, window::Window};

/// Ammount to incriment when modifying values via dragging
const DRAG_INC: f64 = 0.02;

/// Persistend settings
#[derive(Clone)]
struct GuiState {
    /// Live update mode means user input primitive data is continuously updated. Otherwise changes
    /// are not commited until the 'Update' button is pressed.
    pub live_update: bool,
    pub selected_object: Option<Weak<ObjectRef>>,
    pub selected_primitive: Option<Weak<dyn Primitive>>,
}
impl Default for GuiState {
    fn default() -> Self {
        Self {
            live_update: false,
            selected_object: None,
            selected_primitive: None,
        }
    }
}

/// Controller for an [`egui`] immediate-mode gui
pub struct Gui {
    window: Arc<Window>,
    context: egui::Context,
    window_state: egui_winit::State,
    mesh_primitives: Vec<egui::ClippedPrimitive>,
    gui_state: GuiState,
    textures_delta: Vec<TexturesDelta>,
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
            gui_state: Default::default(),
            textures_delta: Default::default(),
        }
    }

    /// Updates context state by winit window event.
    /// Returns `true` if egui wants exclusive use of this event
    /// (e.g. a mouse click on an egui window, or entering text into a text field).
    /// For instance, if you use egui for a game, you want to first call this
    /// and only when this returns `false` pass on the events to your game.
    ///
    /// Note that egui uses `tab` to move focus between elements, so this will always return `true` for tabs.
    pub fn process_event(&mut self, event: &winit::event::WindowEvent<'_>) -> bool {
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
    /// Calling this clears the internal texture delta storage.
    pub fn get_and_clear_textures_delta(&mut self) -> Vec<TexturesDelta> {
        std::mem::take(&mut self.textures_delta)
    }

    pub fn selected_object(&self) -> Option<Weak<ObjectRef>> {
        self.gui_state.selected_object.clone()
    }

    pub fn selected_primitive(&self) -> Option<Weak<dyn Primitive>> {
        self.gui_state.selected_primitive.clone()
    }
}

// Private functions

impl Gui {
    fn objects_window(&mut self, object_collection: &ObjectCollection) {
        // ui layout closure
        let add_contents = |ui: &mut egui::Ui| {
            // object list
            let objects = object_collection.objects();
            for i in 0..objects.len() {
                let current_object = &objects[i];
                let label_text = format!("{} - {}", i, current_object.borrow().name);

                let is_selected = if let Some(object_ref) = &self.gui_state.selected_object {
                    if let Some(selected_object) = object_ref.upgrade() {
                        selected_object.borrow().id() == current_object.borrow().id()
                    } else {
                        debug!("selected object dropped. deselecting object...");
                        self.gui_state.selected_object = None;
                        false
                    }
                } else {
                    false
                };

                if ui.selectable_label(is_selected, label_text).clicked() {
                    self.gui_state.selected_object = Some(Rc::downgrade(current_object));
                };
            }
        };

        // add window to egui context
        egui::Window::new("Objects")
            .resizable(true)
            .vscroll(true)
            .hscroll(true)
            .show(&self.context, add_contents);
    }

    fn bug_test_window(&mut self) {
        let add_contents = |ui: &mut egui::Ui| {
            // TODO TESTING tests GuiRenderer create_texture() functionality for when ImageDelta.pos != None
            // todo add to testing window function and document
            ui.separator();
            if ui.add(Button::new("gui bug test")).clicked() {
                let style = &*self.context.style();
                let mut style = style.clone();
                style.text_styles = [
                    (egui::TextStyle::Heading, FontId::new(20.0, Proportional)),
                    (egui::TextStyle::Body, FontId::new(18.0, Proportional)),
                    (egui::TextStyle::Monospace, FontId::new(14.0, Proportional)),
                    (egui::TextStyle::Button, FontId::new(14.0, Proportional)),
                    (egui::TextStyle::Small, FontId::new(10.0, Proportional)),
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
