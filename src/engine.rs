use crate::camera::Camera;
use crate::config;
use crate::cursor_state::{CursorState, MouseButton};
use crate::gui::Gui;
use crate::helper::anyhow_panic::{anyhow_panic, anyhow_unwrap};
use crate::primitives::{cube::Cube, primitives::PrimitiveCollection, sphere::Sphere};
use crate::renderer::render_manager::RenderManager;
use glam::Vec2;
#[allow(unused_imports)]
use log::{debug, error, info, warn};
use std::sync::Arc;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

pub struct Engine {
    _window: Arc<Window>,
    cursor_state: CursorState,
    window_resize: bool,
    scale_factor: f64,

    camera: Camera,
    primitives: PrimitiveCollection,
    gui: Gui,
    renderer: RenderManager,
}
impl Engine {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let default_resolution = [1000, 700];

        // init window
        let window = Arc::new(
            WindowBuilder::new()
                .with_title(config::ENGINE_NAME)
                .with_inner_size(winit::dpi::LogicalSize::new(
                    f64::from(default_resolution[0]),
                    f64::from(default_resolution[1]),
                ))
                .build(event_loop)
                .expect("failed to instanciate window due to os error"),
        );
        let scale_factor = window.scale_factor();
        let cursor_state = CursorState::new(window.clone());

        // init camera
        let camera = Camera::new(window.inner_size().into());

        // init primitives
        let mut primitives = PrimitiveCollection::default();
        primitives.add_primitive(Sphere::new(glam::Vec3::new(0.0, 1.0, -0.4), 0.4).into());
        primitives.add_primitive(
            Cube::new(glam::Vec3::new(0.0, -1.0, 0.4), glam::Vec3::splat(0.8)).into(),
        );

        // init renderer
        let renderer = anyhow_unwrap(
            RenderManager::new(window.clone(), &primitives),
            "initialize error",
        );

        // init gui
        let gui = Gui::new(&event_loop, window.clone());

        Engine {
            _window: window,
            cursor_state,
            window_resize: false,
            scale_factor,
            camera,
            primitives,
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
            Event::MainEventsCleared => self.process_frame(),
            _ => (),
        }
    }

    /// Process window events and update state
    fn process_input(&mut self, event: WindowEvent) {
        //debug!("winit event: {:?}", event);

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
                // todo instant renderer resize?
                self.window_resize = true;
                self.camera.set_aspect_ratio(new_inner_size.into())
            }
            // dpi change
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                new_inner_size,
            } => {
                // todo instant renderer resize?
                self.scale_factor = scale_factor;
                self.window_resize = true;
                self.camera.set_aspect_ratio((*new_inner_size).into())
            }
            _ => (),
        }
    }

    /// Per frame engine logic and rendering
    fn process_frame(&mut self) {
        // update cursor state
        self.cursor_state.process_frame();

        // update gui
        if let Err(e) = self
            .gui
            .update_frame(&mut self.renderer.gui_renderer_mut(), &mut self.primitives)
        {
            anyhow_panic(&e, "update gui");
        }

        // update camera
        if self.cursor_state.which_dragging() == Some(MouseButton::Left) {
            let delta_cursor: Vec2 =
                (self.cursor_state.position_frame_change() * config::SENSITIVITY_LOOK).as_vec2();
            self.camera
                .rotate(delta_cursor.x.into(), (-delta_cursor.y).into());
        }

        // submit rendering commands
        if let Err(e) =
            self.renderer
                .render_frame(self.window_resize, &self.primitives, &self.gui, self.camera)
        {
            anyhow_panic(&e, "render frame");
        }
        self.window_resize = false;
    }
}
