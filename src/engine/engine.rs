use super::{
    camera::Camera,
    cursor_state::{CursorState, MouseButton},
};
use crate::{
    config,
    helper::anyhow_panic::{anyhow_panic, anyhow_unwrap},
    object::{
        object::Object,
        object_collection::ObjectCollection,
        operation::Operation,
        primitives::{cube::Cube, sphere::Sphere},
    },
    renderer::render_manager::RenderManager,
};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::{rc::Rc, sync::Arc};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

/// Goshenite engine logic
pub struct Engine {
    _window: Arc<Window>,

    // state
    window_resize: bool,
    scale_factor: f64,
    cursor_state: CursorState,
    primitive_lock_on: bool,

    // specialized controllers
    camera: Camera,
    //gui: Gui, bruh
    renderer: RenderManager,

    // model data
    object_collection: ObjectCollection,
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

        let sphere = Sphere::new(glam::Vec3::new(0.0, 0.0, 0.0), 0.5);
        let cube = Cube::new(glam::Vec3::new(-0.2, 0.2, 0.), glam::Vec3::splat(0.8));
        let mut object = Object::new(Rc::new(sphere));
        object.append(Operation::Union, Rc::new(cube));

        let mut object_collection = ObjectCollection::new();
        object_collection.push(object);

        // init renderer
        let renderer = anyhow_unwrap(
            RenderManager::new(window.clone(), &object_collection),
            "initialize renderer",
        );

        // init gui
        //let gui = Gui::new(&event_loop, window.clone()); bruh

        Engine {
            _window: window,

            window_resize: false,
            scale_factor,
            cursor_state,
            primitive_lock_on: false,

            camera,
            //gui, bruh
            renderer,

            object_collection,
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
        trace!("winit event: {:?}", event);

        // egui event handling
        let captured_by_gui = false; //self.gui.process_event(&event); bruh

        // engine event handling
        match event {
            // cursor moved. triggered when cursor is in window or if currently dragging and started in the window (on linux at least)
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_state.set_position(position.into())
            }
            // send mouse button events to input manager
            WindowEvent::MouseInput { state, button, .. } => {
                self.cursor_state
                    .set_click_state(button, state, captured_by_gui)
            }
            WindowEvent::MouseWheel { delta, .. } => self
                .cursor_state
                .accumulate_scroll_delta(delta, captured_by_gui),
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
        // process recieved events for cursor state
        self.cursor_state.process_frame();

        // process gui inputs and update layout
        //if let Err(e) = self bruh
        //    .gui
        //    .update_gui(&mut self.primitive_collection, &mut self.primitive_lock_on)
        //{
        //    anyhow_panic(&e, "update gui");
        //}

        // update camera based on now processed user inputs
        self.update_camera();

        // now that frame processing is done, submit rendering commands
        if let Err(e) = self.renderer.render_frame(self.window_resize, &self.camera) {
            anyhow_panic(&e, "render frame");
        }
        self.window_resize = false;
    }

    fn update_camera(&mut self) {
        // look mode logic
        // NOTE let_chains still unstable: https://github.com/rust-lang/rust/issues/53667
        //let selected_primitive = self.primitive_collection.selected_primitive(); bruh
        //if selected_primitive.is_some() && self.primitive_lock_on {
        //    // set lock on target to selected primitive
        //    let primitive = selected_primitive.expect("if let replacement");
        //    self.camera
        //        .set_lock_on_target(primitive.center().as_dvec3());
        //} else {
        //    // if no primitive selected use arcball mode
        //    self.camera.unset_lock_on_target();
        //}
        self.camera.unset_lock_on_target();

        // left mouse button dragging changes camera orientation
        if self.cursor_state.which_dragging() == Some(MouseButton::Left) {
            self.camera
                .rotate(self.cursor_state.position_frame_change());
        }

        // zoom in/out logic
        let scroll_delta = self.cursor_state.get_and_clear_scroll_delta();
        self.camera.scroll_zoom(scroll_delta.y);
    }
}
