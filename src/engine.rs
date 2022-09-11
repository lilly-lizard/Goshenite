use crate::camera::Camera;
use crate::config;
use crate::cursor_state::{CursorState, MouseButton};
use crate::renderer::render_manager::{RenderManager, RenderManagerError};
use glam::Vec2;
use std::{error, fmt, sync::Arc};
use winit::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
    event_loop::EventLoop,
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

pub struct Engine {
    renderer: RenderManager,
    camera: Camera,
    _window: Arc<Window>,
    window_resize: bool,
    scale_factor: f64,
    cursor_state: CursorState,
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
                .build(&event_loop)
                .unwrap(),
        );

        // init camera
        let camera = Camera::new(init_resolution);

        // init renderer
        let renderer = RenderManager::new(window.clone()).unwrap();

        Engine {
            scale_factor: window.scale_factor(),
            cursor_state: CursorState::new(window.clone()),
            window_resize: false,
            _window: window,
            camera,
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
            // send cursor event to input manager
            Event::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
                ..
            } => self.cursor_state.set_click_state(button, state),
            // cursor moved
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => self.cursor_state.set_new_position(position),
            // cursor entered window
            Event::WindowEvent {
                event: WindowEvent::CursorEntered { .. },
                ..
            } => self.cursor_state.set_in_window_state(true),
            // cursor left window
            Event::WindowEvent {
                event: WindowEvent::CursorLeft { .. },
                ..
            } => self.cursor_state.set_in_window_state(false),
            // window resize
            Event::WindowEvent {
                event: WindowEvent::Resized(new_inner_size),
                ..
            } => {
                self.window_resize = true;
                self.camera.set_aspect_ratio(new_inner_size.into())
            }
            // dpi change
            Event::WindowEvent {
                event:
                    WindowEvent::ScaleFactorChanged {
                        scale_factor,
                        new_inner_size,
                    },
                ..
            } => {
                self.scale_factor = scale_factor;
                self.window_resize = true;
                self.camera.set_aspect_ratio((*new_inner_size).into())
            }
            // per frame logic todo is this called at screen refresh rate?
            Event::MainEventsCleared => self.frame_update().unwrap(), // todo use RedrawRequested?
            _ => (),
        }
    }

    /// Per frame engine logic and rendering
    fn frame_update(&mut self) -> Result<(), EngineError> {
        use EngineError::RendererInvalidated;
        use RenderManagerError::{SurfaceSizeUnsupported, Unrecoverable};

        // update cursor state
        self.cursor_state.frame_update();

        // update camera
        if self.cursor_state.which_dragging() == Some(MouseButton::Left) {
            let delta_cursor: Vec2 =
                (self.cursor_state.position_frame_change() * config::SENSITIVITY_LOOK).as_vec2();
            self.camera
                .rotate(delta_cursor.x.into(), (-delta_cursor.y).into());
        }

        // submit rendering commands
        match self.renderer.render_frame(self.window_resize, self.camera) {
            Err(SurfaceSizeUnsupported { .. }) => (), // todo clamp window inner size
            Err(Unrecoverable(s)) => return Err(RendererInvalidated(s)),
            _ => (),
        };
        self.window_resize = false;

        Ok(())
    }
}
