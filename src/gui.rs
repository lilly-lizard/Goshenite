/// shout out to https://github.com/hakolao/egui_winit_vulkano for a lot of this code
use crate::renderer::gui_renderer::GuiRenderer;
use std::sync::Arc;
use winit::window::Window;

/// Controller for an [`egui`] immediate-mode gui
pub struct Gui {
    window: Arc<Window>,
    context: egui::Context,
    window_state: egui_winit::State,
    primitives: Vec<egui::ClippedPrimitive>,
}
// Public functions
impl Gui {
    /// Creates a new [`Gui`].
    /// * `window`: [`winit`] window
    /// * `max_texture_side`: maximum size of a texture. Query from graphics driver using
    /// [`crate::renderer::render_manager::RenderManager::max_image_array_layers`]
    pub fn new(window: Arc<winit::window::Window>, max_texture_side: usize) -> Self {
        Self {
            window: window.clone(),
            context: Default::default(),
            window_state: egui_winit::State::new(max_texture_side, window.as_ref()),
            primitives: vec![],
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
    pub fn update_frame(&mut self, gui_renderer: &mut GuiRenderer) {
        // begin frame
        let raw_input = self.window_state.take_egui_input(self.window.as_ref());
        self.context.begin_frame(raw_input);

        // set new layout
        self.layout();

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
    /// Sets the layout for the gui
    fn layout(&mut self) {
        egui::Window::new("bruh")
            .resizable(true)
            .vscroll(true)
            .hscroll(true)
            .show(&self.context, |ui| {
                ui.heading("hello egui!");
            });
    }
}
