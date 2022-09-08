use crate::camera::Camera;
use crate::config;
use crate::cursor_state::{CursorState, MouseButton};
use crate::renderer::render_manager::RenderManager;
use glam::Vec2;
use std::sync::Arc;
use winit::event_loop::EventLoop;
use winit::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
    platform::run_return::EventLoopExtRunReturn,
    window::{Window, WindowBuilder},
};

pub struct EngineEntry {
    event_loop: EventLoop<()>,
    engine_instance: Engine,
}
impl EngineEntry {
    pub fn init() -> Self {
        let event_loop = EventLoop::new();
        let engine_instance = Engine::new(&event_loop);
        EngineEntry {
            event_loop,
            engine_instance,
        }
    }

    pub fn start(&mut self) {
        // enter control loop
        self.event_loop.run_return(|event, _, control_flow| {
            self.engine_instance.control_loop(event, control_flow)
        });
    }
}

struct Engine {
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

    pub fn control_loop(&mut self, event: Event<()>, control_flow: &mut ControlFlow) {
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
            Event::MainEventsCleared => self.frame_update(), // todo use RedrawRequested?
            _ => (),
        }
    }

    /// Per frame logic
    fn frame_update(&mut self) {
        // update cursor state
        self.cursor_state.frame_update();

        // update camera
        if self.cursor_state.is_dragging() == Some(MouseButton::Left) {
            let delta_cursor: Vec2 =
                (self.cursor_state.position_frame_change() * config::SENSITIVITY_LOOK).as_vec2();
            self.camera
                .rotate(delta_cursor.x.into(), (-delta_cursor.y).into());
        }

        // submit rendering commands
        self.renderer.render_frame(self.window_resize, self.camera);
        self.window_resize = false;
    }
}
