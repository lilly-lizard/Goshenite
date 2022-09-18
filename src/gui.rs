use crate::renderer::gui_renderer::GuiRenderer;
use egui::epaint::ClippedShape;
use std::sync::Arc;
use winit::window::Window;

// todo remove `pub`s
pub struct Gui {
    window: Arc<Window>,
    pub context: egui::Context,
    pub window_state: egui_winit::State,
    pub shapes: Vec<ClippedShape>,
    pub textures_delta: egui::TexturesDelta,
}
// Public functions
impl Gui {
    // physical_device.properties().max_image_array_layers as usize
    pub fn new(window: Arc<winit::window::Window>, max_image_array_layers: usize) -> Self {
        Self {
            window: window.clone(),
            context: Default::default(),
            window_state: egui_winit::State::new(max_image_array_layers as usize, window.as_ref()),
            shapes: vec![],
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

    pub fn clipped_meshes(&self) -> Vec<egui::ClippedPrimitive> {
        // todo don't use clone because shapes going to be cleared anyway?
        self.context.tessellate(self.shapes.clone())
    }

    pub fn scale_factor(&self) -> f32 {
        self.window_state.pixels_per_point()
    }

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
        self.shapes = shapes;
        self.textures_delta = textures_delta;

        gui_renderer.update_textures(&self.textures_delta);
    }
}
// Private functions
impl Gui {
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
