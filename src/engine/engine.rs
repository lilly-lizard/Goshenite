use super::{
    object::{object_collection::ObjectCollection, operation::Operation},
    primitives::{
        null_primitive::NullPrimitive, primitive::new_primitive_ref,
        primitive_references::PrimitiveReferences,
    },
};
use crate::{
    config,
    helper::anyhow_panic::{anyhow_panic, anyhow_unwrap},
    renderer::render_manager::RenderManager,
    user_interface::camera::Camera,
    user_interface::{
        cursor_state::{CursorState, MouseButton},
        gui::Gui,
    },
};
use glam::Vec3;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::{env, sync::Arc};
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

    // specialized controllers
    camera: Camera,
    gui: Gui,
    renderer: RenderManager,

    // model data
    object_collection: ObjectCollection,
    primitive_references: PrimitiveReferences,
}
impl Engine {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
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

        let scale_factor_override: Option<f64> = match env::var(config::ENV::SCALE_FACTOR) {
            Ok(s) => s.parse::<f64>().ok(),
            _ => None,
        };
        let scale_factor = scale_factor_override.unwrap_or(window.scale_factor());
        let cursor_state = CursorState::new(window.clone());

        let camera = anyhow_unwrap(Camera::new(window.inner_size().into()), "initialize camera");

        // todo put inside object_collection??
        // todo checks in object_collection to make sure that you don't have the same primitive ids across multiple primitive ops
        let mut primitive_references = PrimitiveReferences::new();

        let sphere = primitive_references.new_sphere(Vec3::new(0., 0., 0.), 0.5);
        let cube = primitive_references.new_cube(Vec3::new(-0.2, 0.2, 0.), glam::Vec3::splat(0.8));
        let another_sphere = primitive_references.new_sphere(Vec3::new(0.2, -0.2, 0.), 0.4);

        let mut object_collection = ObjectCollection::new();

        let object =
            object_collection.new_object("Bruh".to_string(), Vec3::new(-1., 1., 0.), cube.clone());
        object
            .borrow_mut()
            .push_op(Operation::Union, sphere.clone());
        object
            .borrow_mut()
            .push_op(Operation::Intersection, another_sphere);

        let another_object = object_collection.new_object(
            "Another Bruh".to_string(),
            Vec3::new(1., -1., 0.),
            sphere.clone(),
        );
        another_object
            .borrow_mut()
            .push_op(Operation::NOP, new_primitive_ref(NullPrimitive {}));

        let renderer = anyhow_unwrap(
            RenderManager::new(window.clone(), &object_collection),
            "initialize renderer",
        );

        let gui = Gui::new(&event_loop, window.clone(), scale_factor as f32);

        Engine {
            _window: window,

            window_resize: false,
            scale_factor,
            cursor_state,

            camera,
            gui,
            renderer,

            object_collection: object_collection,
            primitive_references,
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
        let captured_by_gui = self.gui.process_event(&event).consumed;

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
                self.gui.set_scale_factor(self.scale_factor as f32);
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
        if let Err(e) = self
            .gui
            .update_gui(&self.object_collection, &self.primitive_references)
        {
            anyhow_panic(&e, "update gui");
        }

        // update camera based on now processed user inputs
        self.update_camera();

        // update object buffers
        if let Err(e) = self.renderer.update_object_buffers(
            &self.object_collection,
            self.gui.get_and_clear_objects_delta(),
        ) {
            anyhow_panic(&e, "updating object buffers");
        }

        // update gui renderer
        if let Err(e) = self
            .renderer
            .update_gui_textures(self.gui.get_and_clear_textures_delta())
        {
            anyhow_panic(&e, "updating gui textures");
        }

        // now that frame processing is done, submit rendering commands
        if let Err(e) =
            self.renderer
                .render_frame(self.window_resize, &mut self.gui, &mut self.camera)
        {
            anyhow_panic(&e, "render frame");
        }
        self.window_resize = false;
    }

    fn update_camera(&mut self) {
        // look mode logic
        if let Some(selected_object_ref) = self.gui.selected_object() {
            // set lock on target to selected primitive
            self.camera.set_lock_on_object(selected_object_ref);
        } else {
            // if no primitive selected use arcball mode
            self.camera.unset_lock_on_target();
        }

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
