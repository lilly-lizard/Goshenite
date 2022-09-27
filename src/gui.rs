use crate::primitives::{Primitives, Sphere};
/// shout out to https://github.com/hakolao/egui_winit_vulkano for a lot of this code
use crate::renderer::gui_renderer::GuiRenderer;
use egui::{Button, DragValue};
use glam::Vec3;
use std::sync::Arc;
use winit::window::Window;

// user input values...
#[derive(Clone, Copy, Debug)]
struct InputStorage {
    sphere_radius: f32,
    sphere_center: Vec3,
}
impl Default for InputStorage {
    fn default() -> Self {
        Self {
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
    input_storage: InputStorage,
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
            input_storage: Default::default(),
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

        // primitive list window
        self.primitive_list_window(primitives);

        // new primitive window
        self.add_primitive_window(primitives);

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
    fn primitive_list_window(&mut self, primitives: &Primitives) {
        egui::Window::new("Primitive List")
            .resizable(true)
            .vscroll(true)
            .hscroll(true)
            .show(&self.context, |ui| {
                for sphere in primitives.spheres() {
                    ui.label(format!(
                        "Sphere: radius = {}, center = {}",
                        sphere.radius, sphere.center
                    ));
                }
            });
    }

    fn add_primitive_window(&mut self, primitives: &mut Primitives) {
        egui::Window::new("Add Primitive")
            .resizable(true)
            .vscroll(true)
            .hscroll(true)
            .show(&self.context, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Radius:");
                    ui.add(
                        DragValue::new(&mut self.input_storage.sphere_radius)
                            .speed(0.1)
                            .clamp_range(0..=100),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Center:");
                    ui.add(DragValue::new(&mut self.input_storage.sphere_center.x).speed(0.1));
                    ui.add(DragValue::new(&mut self.input_storage.sphere_center.y).speed(0.1));
                    ui.add(DragValue::new(&mut self.input_storage.sphere_center.z).speed(0.1));
                });
                if ui.add(Button::new("Add")).clicked() {
                    primitives.add_sphere(Sphere::new(
                        self.input_storage.sphere_radius,
                        self.input_storage.sphere_center,
                    ));
                }
            });
    }
}
