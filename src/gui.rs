use crate::primitives::primitives::{Primitive, PrimitiveCollection};
use crate::renderer::gui_renderer::GuiRenderer;
use egui::FontFamily::Proportional;
use egui::{Button, DragValue, FontId};
use std::sync::Arc;
use winit::event_loop::EventLoopWindowTarget;
use winit::window::Window;

// user input values...
#[derive(Default, Clone, Copy, Debug)]
struct InputState {
    selected_primitive: Option<usize>,
    primitive_input: Primitive,
}

/// Controller for an [`egui`] immediate-mode gui
pub struct Gui {
    window: Arc<Window>,
    context: egui::Context,
    window_state: egui_winit::State,
    primitives: Vec<egui::ClippedPrimitive>,
    input_state: InputState,
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
        Self {
            window: window.clone(),
            context,
            window_state: egui_winit::State::new(event_loop),
            primitives: vec![],
            input_state: Default::default(),
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
    pub fn primitives(&self) -> &Vec<egui::ClippedPrimitive> {
        &self.primitives
    }

    /// Returns the scale factor (i.e. window dpi) currently configured for the egui context.
    ///
    /// See [`winit::window::Window::scale_factor`]
    pub fn scale_factor(&self) -> f32 {
        self.window_state.pixels_per_point()
    }

    /// Updates the gui layout and tells the renderer to update any changed resources
    pub fn update_frame(
        &mut self,
        gui_renderer: &mut GuiRenderer,
        primitives: &mut PrimitiveCollection,
    ) -> anyhow::Result<()> {
        // begin frame
        let raw_input = self.window_state.take_egui_input(self.window.as_ref());
        self.context.begin_frame(raw_input);

        // draw primitive editor window
        self.primitives_window(primitives);

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
        self.primitives = self.context.tessellate(shapes);

        // add/free textures resources in the gui renderer. note this must happen here to be
        // certain that this frame's `textures_delta` is processed
        gui_renderer.update_textures(textures_delta)?;

        Ok(())
    }
}
// Private functions
impl Gui {
    fn primitives_window(&mut self, primitives: &mut PrimitiveCollection) {
        // ui layout closure
        let add_contents = |ui: &mut egui::Ui| {
            /// Ammount to incriment when modifying by dragging
            const DRAG_INC: f64 = 0.1;

            // TODO [TESTING] tests GuiRenderer create_texture() functionality for when ImageDelta.pos != None
            // todo add to testing window function and document
            if ui.add(Button::new("gui renderer test")).clicked() {
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

            if let Some(primitive_index) = self.input_state.selected_primitive {
                ui.label(format!("Primitive {}", primitive_index));
            } else {
                // add new primitive
                ui.label("None selected...");
            };

            match self.input_state.primitive_input {
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

            if let Some(primitive_index) = self.input_state.selected_primitive {
                if ui.add(Button::new("Update")).clicked() {
                    // overwrite selected primitive with user data
                    primitives
                        .update_primitive(primitive_index, self.input_state.primitive_input.into());
                }
            } else {
                // todo add new primtive
            }

            ui.separator();
            // TODO CLICKING LOGIC?? frame delay for stuff above to update :''''(

            // if ui
            //     .selectable_label(
            //         self.input_state.selected_primitive.is_none(),
            //         "New primitive",
            //     )
            //     .clicked()
            // {
            //     self.input_state.selected_primitive = None;
            //     todo!("new primitive logic?");
            // }
            let primitives = primitives.primitives();
            for i in 0..primitives.len() {
                let is_selected = if let Some(pi) = self.input_state.selected_primitive {
                    pi == i
                } else {
                    false
                };
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
                if ui.selectable_label(is_selected, label_text).clicked() {
                    self.input_state.selected_primitive = Some(i);
                    self.input_state.primitive_input = primitives[i];
                };
            }
        };
        // add window to egui context
        egui::Window::new("Spheres")
            .resizable(true)
            .vscroll(true)
            .hscroll(true)
            .show(&self.context, add_contents);
    }
}
