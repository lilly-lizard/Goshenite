use crate::camera::Camera;
use crate::config;
use crate::cursor_state::{CursorState, MouseButton};
use crate::gui::Gui;
use crate::renderer::render_manager::{RenderManager, RenderManagerError};
use glam::Vec2;
#[allow(unused_imports)]
use log::{debug, error, info, warn};
use std::{error, fmt, sync::Arc};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

/// Describes different errors encounted by the engine
#[derive(Clone, Debug, PartialEq, Eq)]
enum EngineError {
    /// The renderer has entered or detected an unrecoverable state. Attempting to re-initialize the
    /// render manager may restore funtionality.
    RendererInvalidated(String),
}
impl error::Error for EngineError {}
impl fmt::Display for EngineError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            EngineError::RendererInvalidated(msg) => write!(fmt, "{}", msg),
        }
    }
}

// todo logic for winit
/// Color theme options for the UI.
///
/// Default is [`Theme::Dark`]
/*
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
}
impl Default for Theme {
    fn default() -> Self {
        Self::Dark
    }
}
*/

pub struct Engine {
    _window: Arc<Window>,
    window_resize: bool,
    scale_factor: f64,
    cursor_state: CursorState,
    renderer: RenderManager,
    gui: Gui,
    //theme: Theme,
    camera: Camera,
}
impl Engine {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let init_resolution = [1000, 700];

        // create winit window
        let window = Arc::new(
            WindowBuilder::new()
                .with_title(config::ENGINE_NAME)
                .with_inner_size(winit::dpi::LogicalSize::new(
                    f64::from(init_resolution[0]),
                    f64::from(init_resolution[1]),
                ))
                .build(event_loop)
                .unwrap(),
        );

        // init camera
        let camera = Camera::new(init_resolution);

        // init renderer
        let mut renderer = RenderManager::new(window.clone()).unwrap();
        renderer.primitives.add_sphere(1.0, glam::Vec3::ZERO);

        // init gui
        let gui = Gui::new(window.clone(), renderer.max_image_array_layers() as usize);

        Engine {
            scale_factor: window.scale_factor(),
            cursor_state: CursorState::new(window.clone()),
            window_resize: false,
            _window: window,
            camera,
            gui,
            renderer,
        }
    }

    /// Processes winit events. Pass this function to winit...EventLoop::run_return and think of it as the main loop of the engine.
    pub fn control_flow(&mut self, event: Event<()>, control_flow: &mut ControlFlow) {
        *control_flow = ControlFlow::Poll; // default control flow

        match event {
            // exit the event loop and close application
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            // process window events and update state
            Event::WindowEvent { event, .. } => self.process_input(event),
            // per frame logic todo is this called at screen refresh rate?
            Event::MainEventsCleared => self.process_frame().unwrap(), // todo use RedrawRequested?
            _ => (),
        }
    }

    /// Process window events and update state
    fn process_input(&mut self, event: WindowEvent) {
        debug!("winit event: {:?}", event);

        // egui event handling
        let captured_by_gui = self.gui.process_event(&event);

        match event {
            // cursor moved. triggered when cursor is in window or if currently dragging and started in the window (on linux at least)
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_state.set_position(position.into())
            }
            // send cursor event to input manager
            WindowEvent::MouseInput { state, button, .. } => {
                self.cursor_state
                    .set_click_state(button, state, captured_by_gui)
            }
            // cursor entered window
            WindowEvent::CursorEntered { .. } => self.cursor_state.set_in_window_state(true),
            // cursor left window
            WindowEvent::CursorLeft { .. } => self.cursor_state.set_in_window_state(false),
            // window resize
            WindowEvent::Resized(new_inner_size) => {
                self.window_resize = true;
                self.camera.set_aspect_ratio(new_inner_size.into())
            }
            // dpi change
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                new_inner_size,
            } => {
                self.scale_factor = scale_factor;
                self.window_resize = true;
                self.camera.set_aspect_ratio((*new_inner_size).into())
            }
            _ => (),
        }
    }

    /// Per frame engine logic and rendering
    fn process_frame(&mut self) -> Result<(), EngineError> {
        use EngineError::RendererInvalidated;
        use RenderManagerError::{SurfaceSizeUnsupported, Unrecoverable};

        // update cursor state
        self.cursor_state.process_frame();

        // update gui
        self.gui.update_frame(&mut self.renderer.gui_renderer());

        // update camera
        if self.cursor_state.which_dragging() == Some(MouseButton::Left) {
            let delta_cursor: Vec2 =
                (self.cursor_state.position_frame_change() * config::SENSITIVITY_LOOK).as_vec2();
            self.camera
                .rotate(delta_cursor.x.into(), (-delta_cursor.y).into());
        }

        // submit rendering commands
        match self.renderer.render_frame(
            self.window_resize,
            &self.gui.primitives(),
            self.gui.scale_factor(),
            self.camera,
        ) {
            Err(SurfaceSizeUnsupported { .. }) => (), // todo clamp window inner size
            Err(Unrecoverable(s)) => return Err(RendererInvalidated(s)),
            _ => (),
        };
        self.window_resize = false;

        Ok(())
    }
}
