use crate::primitives::{Primitives, Sphere};
/// shout out to https://github.com/hakolao/egui_winit_vulkano for a lot of this code
use crate::renderer::gui_renderer::GuiRenderer;
use egui::{Button, DragValue};
use egui_winit::egui::Sense;
use glam::Vec3;
use std::sync::Arc;
use winit::window::Window;

// user input values...
#[derive(Clone, Copy, Debug)]
struct InputState {
    selected_sphere: Option<usize>,
    sphere_radius: f32,
    sphere_center: Vec3,
}
impl Default for InputState {
    fn default() -> Self {
        Self {
            selected_sphere: None,
            sphere_radius: 1.0,
            sphere_center: Vec3::ZERO,
        }
    }
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
    pub fn new(window: Arc<winit::window::Window>, max_texture_side: usize) -> Self {
        let context = egui::Context::default();
        context.set_style(egui::Style {
            // disable sentance wrap by default (horizontal scroll instead)
            wrap: Some(false),
            ..Default::default()
        });
        Self {
            window: window.clone(),
            context,
            window_state: egui_winit::State::new(max_texture_side, window.as_ref()),
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
    pub fn update_frame(&mut self, gui_renderer: &mut GuiRenderer, primitives: &mut Primitives) {
        // begin frame
        let raw_input = self.window_state.take_egui_input(self.window.as_ref());
        self.context.begin_frame(raw_input);

        self.spheres_window(primitives);

        // end frame
        let egui::FullOutput {
            platform_output,
            needs_repaint: _r,
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
        gui_renderer.update_textures(&textures_delta);
    }
}
// Private functions
impl Gui {
    fn spheres_window(&mut self, primitives: &mut Primitives) {
        egui::Window::new("Spheres")
            .resizable(true)
            .vscroll(true)
            .hscroll(true)
            .show(&self.context, |ui| {
                if let Some(sphere_index) = self.input_state.selected_sphere {
                    ui.label(format!("Sphere {}", sphere_index));
                } else {
                    ui.label("New sphere");
                };

                /// Ammount to incriment when modifying by dragging
                const DRAG_INC: f64 = 0.1;
                ui.horizontal(|ui| {
                    ui.label("Center:");
                    ui.add(DragValue::new(&mut self.input_state.sphere_center.x).speed(DRAG_INC));
                    ui.add(DragValue::new(&mut self.input_state.sphere_center.y).speed(DRAG_INC));
                    ui.add(DragValue::new(&mut self.input_state.sphere_center.z).speed(DRAG_INC));
                });
                ui.horizontal(|ui| {
                    ui.label("Radius:");
                    ui.add(
                        DragValue::new(&mut self.input_state.sphere_radius)
                            .speed(DRAG_INC)
                            .clamp_range(0..=100),
                    );
                });

                if let Some(sphere_index) = self.input_state.selected_sphere {
                    if ui
                        .add(Button::new(format!("Update {}", sphere_index)))
                        .clicked()
                    {
                        primitives.update_sphere(
                            sphere_index,
                            Sphere::new(
                                self.input_state.sphere_radius,
                                self.input_state.sphere_center,
                            ),
                        );
                    }
                } else {
                    if ui.add(Button::new("Add")).clicked() {
                        primitives.add_sphere(Sphere::new(
                            self.input_state.sphere_radius,
                            self.input_state.sphere_center,
                        ));
                    }
                }

                ui.separator();

                // TODO CLICKING LOGIC?? frame delay for stuff above to update :''''(
                if ui
                    .selectable_label(self.input_state.selected_sphere.is_none(), "New sphere")
                    .clicked()
                {
                    self.input_state.selected_sphere = None;
                }
                let spheres = primitives.spheres();
                for i in 0..spheres.len() {
                    let selected = if let Some(si) = self.input_state.selected_sphere {
                        si == i
                    } else {
                        false
                    };
                    if ui
                        .selectable_label(
                            selected,
                            format!(
                                "{} Sphere: radius = {}, center = {}",
                                i, spheres[i].radius, spheres[i].center
                            ),
                        )
                        .clicked()
                    {
                        self.input_state.selected_sphere = Some(i);
                    };
                }
            });
    }
}
