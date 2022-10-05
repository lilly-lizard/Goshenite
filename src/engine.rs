use crate::camera::Camera;
use crate::config;
use crate::cursor_state::{CursorState, MouseButton};
use crate::gui::Gui;
use crate::helper::anyhow_panic::{anyhow_panic, anyhow_unwrap};
use crate::primitives::{cube::Cube, primitive_collection::PrimitiveCollection, sphere::Sphere};
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
    primitive_collection: PrimitiveCollection,
    gui: Gui,
    renderer: RenderManager,
}
impl Engine {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        // init window
        let mut window_builder = WindowBuilder::new().with_title(config::ENGINE_NAME);
        if config::START_MAXIMIZED {
            window_builder = window_builder.with_maximized(true);
        } else {
            window_builder = window_builder.with_inner_size(winit::dpi::LogicalSize::new(
                config::DEFAULT_WINDOW_SIZE[0],
                config::DEFAULT_WINDOW_SIZE[1],
            ));
        }
        let window = Arc::new(
            window_builder
                .build(event_loop)
                .expect("failed to instanciate window due to os error"),
        );
        let scale_factor = window.scale_factor();
        let cursor_state = CursorState::new(window.clone());

        // init camera
        let camera = anyhow_unwrap(Camera::new(window.inner_size().into()), "initialize camera");

        // init primitives
        let mut primitive_collection = PrimitiveCollection::default();
        primitive_collection.add_primitive(Sphere::new(glam::Vec3::new(0.0, 0.0, 0.0), 0.4).into());
        primitive_collection.add_primitive(
            Cube::new(glam::Vec3::new(0.0, -1.5, 0.5), glam::Vec3::splat(0.8)).into(),
        );

        // init renderer
        let renderer = anyhow_unwrap(
            RenderManager::new(window.clone(), &primitive_collection),
            "initialize renderer",
        );

        // init gui
        let gui = Gui::new(&event_loop, window.clone());

        Engine {
            _window: window,
            cursor_state,
            window_resize: false,
            scale_factor,
            camera,
            primitive_collection,
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
            // per frame logic
            Event::MainEventsCleared => self.process_frame(),
            _ => (),
        }
    }

    /// Process window events and update state
    fn process_input(&mut self, event: WindowEvent) {
        if config::PER_FRAME_DEBUG_LOGS {
            debug!("winit event: {:?}", event);
        }

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
    fn process_frame(&mut self) {
        // input processing...

        // update cursor state
        self.cursor_state.process_frame();

        // process gui state and update layout
        if let Err(e) = self.gui.update_gui_layout(
            &mut self.renderer.gui_renderer_mut(),
            &mut self.primitive_collection,
            &mut self.camera,
        ) {
            anyhow_panic(&e, "update gui");
        }

        // engine processing...

        // update camera
        self.camera.process_frame(&self.cursor_state);

        // submit rendering commands
        if let Err(e) = self.renderer.render_frame(
            self.window_resize,
            &self.camera,
            &self.primitive_collection,
            &self.gui,
        ) {
            anyhow_panic(&e, "render frame");
        }
        self.window_resize = false;
    }
}
