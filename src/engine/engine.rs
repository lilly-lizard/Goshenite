use super::{
    config_engine,
    object::{
        object_collection::ObjectCollection, objects_delta::ObjectsDelta, operation::Operation,
    },
    primitives::null_primitive::NullPrimitive,
    render_thread::{start_render_thread, RenderThreadCommand, RenderThreadUpdaters},
};
use crate::{
    config,
    helper::anyhow_panic::anyhow_unwrap,
    renderer::render_manager::RenderManager,
    user_interface::camera::Camera,
    user_interface::{
        cursor::{Cursor, MouseButton},
        gui::Gui,
    },
};
use glam::{Quat, Vec3};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::{env, sync::Arc, thread::JoinHandle, time::Instant};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

/// Goshenite engine logic
pub struct Engine {
    _window: Arc<Window>,

    // state
    scale_factor: f64,
    cursor_state: Cursor,
    object_collection: ObjectCollection,
    frame_number: u64,

    // controllers
    camera: Camera,
    gui: Gui,

    // render thread
    render_thread_handle: JoinHandle<()>,
    render_thread_updaters: RenderThreadUpdaters,
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

        let cursor_state = Cursor::new();

        let camera = anyhow_unwrap(Camera::new(window.inner_size().into()), "initialize camera");

        let mut renderer = anyhow_unwrap(
            RenderManager::new(window.clone(), scale_factor as f32),
            "initialize renderer",
        );
        anyhow_unwrap(renderer.update_camera(&camera), "init renderer camera");

        let gui = Gui::new(&event_loop, window.clone(), scale_factor as f32);

        let mut object_collection = ObjectCollection::new();

        // TESTING OBJECTS START

        let sphere = object_collection.primitive_references_mut().create_sphere(
            Vec3::new(0., 0., 0.),
            Quat::IDENTITY,
            0.5,
        );
        let cube = object_collection.primitive_references_mut().create_cube(
            Vec3::new(-0.2, 0.2, 0.),
            Quat::IDENTITY,
            glam::Vec3::splat(0.8),
        );
        let another_sphere = object_collection.primitive_references_mut().create_sphere(
            Vec3::new(0.2, -0.2, 0.),
            Quat::IDENTITY,
            0.83,
        );

        let object = object_collection.new_object(
            "Bruh".to_string(),
            Vec3::new(-0.2, 0.2, 0.),
            cube.clone(),
        );
        object
            .borrow_mut()
            .push_op(Operation::Union, sphere.clone());
        object
            .borrow_mut()
            .push_op(Operation::Intersection, another_sphere);

        let another_object = object_collection.new_object(
            "Another Bruh".to_string(),
            Vec3::new(0.2, -0.2, 0.),
            sphere.clone(),
        );
        another_object
            .borrow_mut()
            .push_op(Operation::Union, NullPrimitive::new_ref());

        let mut objects_delta = ObjectsDelta::default();
        objects_delta.update.insert(object.borrow().id());
        objects_delta.update.insert(another_object.borrow().id());

        anyhow_unwrap(
            renderer.update_object_buffers(&object_collection, objects_delta),
            "update object buffers",
        );

        // TESTING OBJECTS END

        // start render thread
        let (render_thread_handle, render_thread_updaters) = start_render_thread(renderer);

        Engine {
            _window: window,

            scale_factor,
            cursor_state,
            object_collection,
            frame_number: 0,

            camera,
            gui,

            render_thread_handle,
            render_thread_updaters,
        }
    }

    /// Processes winit events. Pass this function to winit...EventLoop::run_return and think of it as the main loop of the engine.
    pub fn control_flow(&mut self, event: Event<()>, control_flow: &mut ControlFlow) {
        match event {
            // exit the event loop and close application
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                info!("close requested by window");
                *control_flow = ControlFlow::Exit;

                self.stop_render_thread();
            }

            // process window events and update state
            Event::WindowEvent { event, .. } => self.process_input(event),

            // per frame logic
            Event::MainEventsCleared => match *control_flow {
                ControlFlow::ExitWithCode(_) => (), // don't bother if we're quitting anyway
                _ => {
                    let quit = self.process_frame();

                    if quit {
                        info!("close requested by frame processing");
                        *control_flow = ControlFlow::Exit;

                        self.stop_render_thread();
                    }
                }
            },
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

            // send mouse button events to cursor state
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
                self.update_window_inner_size(new_inner_size);
            }

            // dpi change
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                new_inner_size,
            } => {
                self.set_scale_factor(scale_factor);
                self.update_window_inner_size(*new_inner_size);
            }

            WindowEvent::ThemeChanged(winit_theme) => {
                self.gui.set_theme_winit(winit_theme);
            }
            _ => (),
        }
    }

    fn update_window_inner_size(&mut self, new_inner_size: winit::dpi::PhysicalSize<u32>) {
        self.camera.set_aspect_ratio(new_inner_size.into());
        self.render_thread_updaters.set_window_just_resized_flag();
    }

    fn set_scale_factor(&mut self, scale_factor: f64) {
        self.scale_factor = scale_factor;
        self.gui.set_scale_factor(self.scale_factor as f32);
        self.render_thread_updaters
            .set_scale_factor(scale_factor as f32);
    }

    /// Per frame engine logic and rendering.
    /// Returns true if we want to quit.
    fn process_frame(&mut self) -> bool {
        // process recieved events for cursor state
        self.cursor_state.process_frame();

        // process gui inputs and update layout
        if let Some(cursor_icon) = self.cursor_state.get_cursor_icon() {
            self.gui.set_cursor_icon(cursor_icon);
        }
        anyhow_unwrap(
            self.gui
                .update_gui(&mut self.object_collection, &mut self.camera),
            "update gui",
        );

        // update camera based on now processed user inputs
        self.update_camera();
        self.render_thread_updaters
            .update_camera(self.camera.clone());

        // update object buffers todo better objects delta
        // anyhow_unwrap(
        //     self.renderer.update_object_buffers(
        //         &self.object_collection,
        //         self.gui.get_and_clear_objects_delta(),
        //     ),
        //     "update object buffers",
        // );

        // update gui renderer
        let textures_delta = self.gui.get_and_clear_textures_delta();
        self.render_thread_updaters
            .update_gui_textures(textures_delta);
        let gui_primitives = self.gui.mesh_primitives().clone();
        self.render_thread_updaters
            .set_gui_primitives(gui_primitives);

        // now that frame processing is done, tell renderer to render a frame
        let render_thread_send_res = self
            .render_thread_updaters
            .set_render_thread_command(RenderThreadCommand::RenderFrame);

        // quit if the render thread is killed
        if let Err(e) = render_thread_send_res {
            warn!("process_frame: render thread already stopped ({})", e);
            return true;
        }

        self.frame_number += 1;
        false
    }

    fn update_camera(&mut self) {
        // if no primitive selected use arcball mode
        if self.gui.selected_object().is_none() {
            self.camera.unset_lock_on_target();
        }

        // left mouse button dragging changes camera orientation
        if self.cursor_state.which_dragging() == Some(MouseButton::Left) {
            self.camera
                .rotate_from_cursor_delta(self.cursor_state.position_frame_change());
        }

        // zoom in/out logic
        let scroll_delta = self.cursor_state.get_and_clear_scroll_delta();
        self.camera.scroll_zoom(scroll_delta.y);
    }

    fn stop_render_thread(&self) {
        debug!("sending quit command to render thread...");
        let _render_thread_send_res = self
            .render_thread_updaters
            .set_render_thread_command(RenderThreadCommand::Quit);

        debug!(
            "waiting for render thread to quit (timeout = {:.2}s)",
            config_engine::RENDER_THREAD_WAIT_TIMEOUT_SECONDS
        );
        let render_thread_timeout_begin = Instant::now();
        let timeout_millis = (config_engine::RENDER_THREAD_WAIT_TIMEOUT_SECONDS * 1_000.) as u128;
        loop {
            let render_thread_quit = self.render_thread_handle.is_finished();
            if render_thread_quit {
                debug!("render thread quit.");
                break;
            }
            if render_thread_timeout_begin.elapsed().as_millis() > timeout_millis {
                error!(
                    "render thread hanging longer than timeout of {:.2}s. continuing now...",
                    config_engine::RENDER_THREAD_WAIT_TIMEOUT_SECONDS
                );
                break;
            }
        }
    }
}

// ~~ Render Thread ~~

// ~~ Engine Error ~~

#[derive(Clone, Copy, Debug)]
pub enum EngineError {
    RenderThreadClosedPrematurely,
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::RenderThreadClosedPrematurely => {
                write!(f, "render thread was closed prematurely")
            }
        }
    }
}

impl std::error::Error for EngineError {}
