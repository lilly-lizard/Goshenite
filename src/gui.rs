use crate::primitives::{primitive::Primitive, primitive_collection::PrimitiveCollection};
use egui::{
    Button, Checkbox, ComboBox, DragValue, FontFamily::Proportional, FontId, Sense, TexturesDelta,
};
#[allow(unused_imports)]
use log::{debug, error, info, warn};
use std::sync::Arc;
use winit::{event_loop::EventLoopWindowTarget, window::Window};

/// Primitive editor window state e.g. user input
#[derive(Clone, Copy, Debug)]
struct PrimitiveEditorState {
    /// Contains user input data for primitive editor
    pub primitive_input: Primitive,
    /// Live update mode means user input primitive data is continuously updated
    pub live_update: bool,
}
impl Default for PrimitiveEditorState {
    fn default() -> Self {
        Self {
            primitive_input: Primitive::Null,
            live_update: false,
        }
    }
}

/// Controller for an [`egui`] immediate-mode gui
pub struct Gui {
    window: Arc<Window>,
    context: egui::Context,
    window_state: egui_winit::State,
    mesh_primitives: Vec<egui::ClippedPrimitive>,
    primitive_editor_state: PrimitiveEditorState,
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
            primitive_editor_state: Default::default(),
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

    /// Returns the scale factor (i.e. window dpi) currently configured for the egui context.
    ///
    /// See [`winit::window::Window::scale_factor`]
    pub fn scale_factor(&self) -> f32 {
        self.window_state.pixels_per_point()
    }

    /// Updates the gui layout and tells the renderer to update any changed resources
    /// * `primitive_collection` - collection for the 'Primitive Editor' window to edit
    /// * `primitive_lock_on` - value that the lock-on setting will be written to
    pub fn update_gui(
        &mut self,
        primitive_collection: &mut PrimitiveCollection,
        primitive_lock_on: &mut bool,
    ) -> anyhow::Result<()> {
        // begin frame
        let raw_input = self.window_state.take_egui_input(self.window.as_ref());
        self.context.begin_frame(raw_input);

        // draw primitive editor window
        self.primitives_window(primitive_collection, primitive_lock_on);

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
    /// Calling this clears the internal texture delta storage, so be sure appropriate renderer
    /// updates are done after calling this.
    pub fn get_and_clear_textures_delta(&mut self) -> Vec<TexturesDelta> {
        std::mem::take(&mut self.textures_delta)
    }
}
// Private functions
impl Gui {
    fn primitives_window(
        &mut self,
        primitive_collection: &mut PrimitiveCollection,
        primitive_lock_on: &mut bool,
    ) {
        // ui layout closure
        let add_contents = |ui: &mut egui::Ui| {
            /// Ammount to incriment when modifying by dragging
            const DRAG_INC: f64 = 0.02;

            // persistent state
            let PrimitiveEditorState {
                primitive_input,
                live_update,
            } = &mut self.primitive_editor_state;
            let selected_primitive = primitive_collection.selected_primitive_index();

            if let Some(primitive_index) = selected_primitive {
                // status
                ui.heading(format!("Modify primitive {}", primitive_index));

                // lock-on tick-box
                ui.add(Checkbox::new(primitive_lock_on, "Enable lock-on"));

                // update primitive buttons
                let mut update_primitive = *live_update;
                ui.horizontal(|ui| {
                    // 'Update' button (disabled in 'Live update' mode)
                    update_primitive |= ui
                        .add(Button::new("Update").sense(if *live_update {
                            // disable if 'Live update' mode is on
                            Sense::hover()
                        } else {
                            Sense::click()
                        }))
                        .clicked();
                    // 'Live update' checkbox means the primitive data gets constantly updated
                    ui.add(Checkbox::new(&mut *live_update, "Live update"));
                });
                if update_primitive {
                    // overwrite selected primitive with user data
                    if let Err(e) =
                        primitive_collection.update_primitive(primitive_index, *primitive_input)
                    {
                        warn!("could not update primitive due to: {}", e);
                    }
                }
            } else {
                // status
                ui.heading("New primitive");

                ui.horizontal(|ui| {
                    // new primitive button
                    if ui
                        .add(
                            Button::new("Add").sense(if *primitive_input == Primitive::Null {
                                // disable if primitive type == Null
                                Sense::hover()
                            } else {
                                Sense::click()
                            }),
                        )
                        .clicked()
                    {
                        primitive_collection.add_primitive(*primitive_input);
                    }

                    // dropdown menu to select primitive type
                    ComboBox::from_label("Primitive type")
                        .selected_text(primitive_input.type_name())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut *primitive_input,
                                Primitive::Sphere(Default::default()),
                                "Sphere",
                            );
                            ui.selectable_value(
                                &mut *primitive_input,
                                Primitive::Cube(Default::default()),
                                "Cube",
                            );
                        });
                });
            };

            // user data input section
            match *primitive_input {
                Primitive::Sphere(ref mut s) => {
                    ui.horizontal(|ui| {
                        ui.label("Center:");
                        ui.add(DragValue::new(&mut s.center.x).speed(DRAG_INC));
                        ui.add(DragValue::new(&mut s.center.y).speed(DRAG_INC));
                        ui.add(DragValue::new(&mut s.center.z).speed(DRAG_INC));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Radius:");
                        ui.add(
                            DragValue::new(&mut s.radius)
                                .speed(DRAG_INC)
                                .clamp_range(0..=100),
                        );
                    });
                }
                Primitive::Cube(ref mut c) => {
                    ui.horizontal(|ui| {
                        ui.label("Center:");
                        ui.add(DragValue::new(&mut c.center.x).speed(DRAG_INC));
                        ui.add(DragValue::new(&mut c.center.y).speed(DRAG_INC));
                        ui.add(DragValue::new(&mut c.center.z).speed(DRAG_INC));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Dimensions:");
                        ui.add(DragValue::new(&mut c.dimensions.x).speed(DRAG_INC));
                        ui.add(DragValue::new(&mut c.dimensions.y).speed(DRAG_INC));
                        ui.add(DragValue::new(&mut c.dimensions.z).speed(DRAG_INC));
                    });
                }
                Primitive::Null => (),
            };

            ui.separator();

            // new primitive button
            if ui
                .selectable_label(selected_primitive.is_none(), "New primitive")
                .clicked()
            {
                primitive_collection.unset_selected_primitive();
                *primitive_input = Primitive::Null;
            }
            // primitive list
            // todo performance hit when list becomes too big (https://github.com/emilk/egui#cpu-usage) try only laying out part in view
            let primitives = primitive_collection.primitives();
            let mut new_selected_primitive: Option<usize> = None;
            for i in 0..primitives.len() {
                // label text depending on primitive type
                let label_text = match primitives[i] {
                    Primitive::Sphere(s) => {
                        format!("{} Sphere: center = {}, radius = {}", i, s.center, s.radius)
                    }
                    Primitive::Cube(c) => format!(
                        "{} Cube: center = {}, radius = {}",
                        i, c.center, c.dimensions
                    ),
                    Primitive::Null => format!("{} Null", 1),
                };
                // check if this primitive is selected
                let is_selected = if let Some(pi) = selected_primitive {
                    pi == i
                } else {
                    false
                };
                // selectable label
                if ui.selectable_label(is_selected, label_text).clicked() {
                    new_selected_primitive = Some(i);
                    *primitive_input = primitives[i];
                };
            }
            // if a primitive from the list was selected, tell primitive_collection
            if let Some(index) = new_selected_primitive {
                // if index is invalid, no harm done
                let _err = primitive_collection.set_selected_primitive(index);
            }

            // TODO [TESTING] tests GuiRenderer create_texture() functionality for when ImageDelta.pos != None
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
        // add window to egui context
        egui::Window::new("Primitive Editor")
            .resizable(true)
            .vscroll(true)
            .hscroll(true)
            .show(&self.context, add_contents);
    }
}
