use crate::engine::{
    object::{object::ObjectRef, object_collection::ObjectCollection},
    primitives::{
        primitive_ref_types::PrimitiveRefType, primitive_references::PrimitiveReferences,
    },
};
use egui::{
    Button, Checkbox, ComboBox, DragValue, FontFamily::Proportional, FontId, RichText, Sense,
    TextStyle, TexturesDelta,
};
use egui_winit::EventResponse;
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
    pub selected_primitive_op_index: Option<usize>,
}
impl GuiState {
    #[inline]
    pub fn deselect_object(&mut self) {
        self.selected_object = None;
        self.selected_primitive_op_index = None;
    }
}
impl Default for GuiState {
    fn default() -> Self {
        Self {
            live_update: false,
            selected_object: None,
            selected_primitive_op_index: None,
        }
    }
}

/// Controller for an [`egui`] immediate-mode gui
pub struct Gui {
    window: Arc<Window>,
    context: egui::Context,
    window_state: egui_winit::State,
    mesh_primitives: Vec<egui::ClippedPrimitive>,
    state: GuiState,
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
            state: Default::default(),
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
    /// Calling this clears the internal texture delta storage.
    pub fn get_and_clear_textures_delta(&mut self) -> Vec<TexturesDelta> {
        std::mem::take(&mut self.textures_delta)
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
            // object list
            let objects = object_collection.objects();
            for i in 0..objects.len() {
                let current_object = &objects[i];
                let label_text = RichText::new(format!("{} - {}", i, current_object.borrow().name))
                    .text_style(TextStyle::Monospace);

                let is_selected = if let Some(object_ref) = &self.state.selected_object {
                    if let Some(selected_object) = object_ref.upgrade() {
                        selected_object.borrow().id() == current_object.borrow().id()
                    } else {
                        debug!("selected object dropped. deselecting object...");
                        self.state.deselect_object();
                        false
                    }
                } else {
                    false
                };

                if ui.selectable_label(is_selected, label_text).clicked() {
                    if !is_selected {
                        self.state.selected_object = Some(Rc::downgrade(current_object));
                        self.state.selected_primitive_op_index = None;
                    }
                }
            }
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
            let selected_object_ref = match &self.state.selected_object {
                Some(o) => o.clone(),
                None => {
                    ui.label(no_object_text);
                    return;
                }
            };
            let selected_object = match selected_object_ref.upgrade() {
                Some(o) => o,
                None => {
                    debug!("selected object dropped. deselecting object...");
                    self.state.deselect_object();
                    ui.label(no_object_text);
                    return;
                }
            };
            let selected_object = selected_object.borrow();

            ui.heading(format!("{}", selected_object.name));

            // primitive op editor
            if let Some(selected_primitive_op_index) = self.state.selected_primitive_op_index {
                if selected_primitive_op_index < selected_object.primitive_ops.len() {
                    let selected_primitive_op =
                        &selected_object.primitive_ops[selected_primitive_op_index];
                    let primitive_type = PrimitiveRefType::from_name(
                        selected_primitive_op.prim.borrow().type_name(),
                    );

                    match primitive_type {
                        PrimitiveRefType::Sphere => {
                            let sphere_id = selected_primitive_op.prim.borrow().id();
                            let sphere_ref = primitive_references.get_sphere(sphere_id)
                                .expect("primitive collection doesn't contain primitive id from object op. this is a bug!");
                            let mut sphere = sphere_ref.borrow_mut();

                            ui.separator();
                            ui.label("Edit Sphere");
                            ui.horizontal(|ui| {
                                ui.label("Center:");
                                ui.add(DragValue::new(&mut sphere.center.x).speed(DRAG_INC));
                                ui.add(DragValue::new(&mut sphere.center.y).speed(DRAG_INC));
                                ui.add(DragValue::new(&mut sphere.center.z).speed(DRAG_INC));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Radius:");
                                ui.add(
                                    DragValue::new(&mut sphere.radius)
                                        .speed(DRAG_INC)
                                        .clamp_range(0..=100),
                                );
                            });
                        }
                        PrimitiveRefType::Cube => {
                            let cube_id = selected_primitive_op.prim.borrow().id();
                            let cube_ref = primitive_references.get_cube(cube_id)
                                .expect("primitive collection doesn't contain primitive id from object op. this is a bug!");
                            let mut cube = cube_ref.borrow_mut();

                            ui.separator();
                            ui.label("Edit Cube");
                            ui.horizontal(|ui| {
                                ui.label("Center:");
                                ui.add(DragValue::new(&mut cube.center.x).speed(DRAG_INC));
                                ui.add(DragValue::new(&mut cube.center.y).speed(DRAG_INC));
                                ui.add(DragValue::new(&mut cube.center.z).speed(DRAG_INC));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Dimensions:");
                                ui.add(DragValue::new(&mut cube.dimensions.x).speed(DRAG_INC));
                                ui.add(DragValue::new(&mut cube.dimensions.y).speed(DRAG_INC));
                                ui.add(DragValue::new(&mut cube.dimensions.z).speed(DRAG_INC));
                            });
                        }
                        _ => {
                            ui.label(format!(
                                "Primitive Type: {}",
                                selected_primitive_op.prim.borrow().type_name()
                            ));
                        }
                    }
                }
            }

            ui.separator();

            // primitive op list
            for i in 0..selected_object.primitive_ops.len() {
                let current_primitive_op = &selected_object.primitive_ops[i];

                let label_text = RichText::new(format!(
                    "{} - {} {}",
                    i,
                    current_primitive_op.op.name(),
                    current_primitive_op.prim.borrow().type_name()
                ))
                .text_style(TextStyle::Monospace);

                let is_selected = if let Some(index) = self.state.selected_primitive_op_index {
                    index == i
                } else {
                    false
                };
                if ui.selectable_label(is_selected, label_text).clicked() {
                    self.state.selected_primitive_op_index = Some(i);
                }
            }
        };

        // add window to egui context
        egui::Window::new("Object Editor")
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
