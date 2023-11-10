use super::{
    commands::CommandWithSource,
    config_engine,
    object::{
        object::ObjectId, object_collection::ObjectCollection, operation::Operation,
        primitive_op::PrimitiveOpId,
    },
    primitives::{cube::Cube, null_primitive::NullPrimitive, primitive::Primitive, sphere::Sphere},
    render_thread::{start_render_thread, RenderThreadChannels, RenderThreadCommand},
};
use crate::{
    config,
    helper::anyhow_panic::anyhow_unwrap,
    renderer::{element_id_reader::ElementAtPoint, render_manager::RenderManager},
    user_interface::camera::Camera,
    user_interface::{
        cursor::{Cursor, CursorEvent, MouseButton},
        gui::Gui,
    },
};
use glam::{Quat, Vec3};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use single_value_channel::NoReceiverError;
use std::{
    collections::VecDeque,
    env,
    sync::{mpsc::SendError, Arc},
    thread::JoinHandle,
    time::Instant,
};
use winit::{
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

// engine_instance sub-modules (files in engine_instance directory)
mod commands_impl;

/// Goshenite engine logic
pub struct EngineInstance {
    window: Arc<Window>,

    // state
    pub(super) scale_factor: f64,
    pub(super) object_collection: ObjectCollection, // note: some engine code written on the assumtion that there is only one object collection
    pub(super) main_thread_frame_number: u64,
    pub(super) pending_commands: VecDeque<CommandWithSource>,
    pub(super) selected_object_id: Option<ObjectId>,
    pub(super) selected_primitive_op_id: Option<PrimitiveOpId>,

    // controllers
    pub(super) cursor: Cursor,
    pub(super) camera: Camera,
    pub(super) gui: Gui,

    // render thread
    render_thread_handle: JoinHandle<()>,
    render_thread_channels: RenderThreadChannels,
}

impl EngineInstance {
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

        let cursor = Cursor::new();

        let camera = anyhow_unwrap(Camera::new(window.inner_size().into()), "initialize camera");

        let init_renderer_res = RenderManager::new(window.clone(), scale_factor as f32);
        let mut renderer = anyhow_unwrap(init_renderer_res, "initialize renderer");

        let renderer_update_camera_res = renderer.update_camera(&camera);
        anyhow_unwrap(renderer_update_camera_res, "init renderer camera");

        let gui = Gui::new(&event_loop, scale_factor as f32);

        let mut object_collection = ObjectCollection::new();

        // TESTING OBJECTS START

        object_testing(&mut object_collection, &mut renderer);

        // TESTING OBJECTS END

        // start render thread
        let (render_thread_handle, render_thread_channels) = start_render_thread(renderer);

        EngineInstance {
            window,

            scale_factor,
            object_collection,
            main_thread_frame_number: 0,
            pending_commands: VecDeque::new(),
            selected_object_id: None,
            selected_primitive_op_id: None,

            cursor,
            camera,
            gui,

            render_thread_handle,
            render_thread_channels,
        }
    }

    /// The main loop of the engine thread. Processes winit events. Pass this function to EventLoop::run_return.
    pub fn control_flow(&mut self, event: Event<()>, control_flow: &mut ControlFlow) {
        match *control_flow {
            ControlFlow::ExitWithCode(_) => return, // don't do any more processing if we're quitting
            _ => (),
        }

        match event {
            // initialize the window
            Event::NewEvents(StartCause::Init) => {
                // note: window initialization (and thus swapchain init too) is done here because of certain platform epecific behaviour e.g. https://github.com/rust-windowing/winit/issues/2051
                // todo init!
            }

            // exit the event loop and close application
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                info!("close requested by window");

                // quit
                *control_flow = ControlFlow::Exit;
                self.stop_render_thread();
            }

            // process window events and update state
            Event::WindowEvent { event, .. } => {
                let process_input_res = self.process_input(event);

                if let Err(e) = process_input_res {
                    error!("error while processing input: {}", e);

                    // quit
                    *control_flow = ControlFlow::Exit;
                    self.stop_render_thread();
                }
            }

            // per frame logic
            Event::MainEventsCleared => {
                let process_frame_res = self.per_frame_processing();

                if let Err(e) = process_frame_res {
                    error!("error during per-frame processing: {}", e);

                    // quit
                    *control_flow = ControlFlow::Exit;
                    self.stop_render_thread();
                }
            }
            _ => (),
        }
    }

    /// Process window events and update state
    fn process_input(&mut self, event: WindowEvent) -> Result<(), EngineError> {
        trace!("winit event: {:?}", event);

        // egui event handling
        let captured_by_egui = self.gui.process_event(&event).consumed;

        // engine event handling
        match event {
            // cursor moved. triggered when cursor is in window or if currently dragging and started in the window (on linux at least)
            WindowEvent::CursorMoved { position, .. } => self.cursor.set_position(position.into()),

            // send mouse button events to cursor state
            WindowEvent::MouseInput { state, button, .. } => {
                self.cursor.set_click_state(button, state, captured_by_egui)
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.cursor.accumulate_scroll_delta(delta, captured_by_egui)
            }

            // cursor entered window
            WindowEvent::CursorEntered { .. } => self.cursor.set_in_window_state(true),

            // cursor left window
            WindowEvent::CursorLeft { .. } => self.cursor.set_in_window_state(false),

            // window resize
            WindowEvent::Resized(new_inner_size) => {
                self.update_window_inner_size(new_inner_size)?;
            }

            // dpi change
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                new_inner_size,
            } => {
                self.set_scale_factor(scale_factor)?;
                self.update_window_inner_size(*new_inner_size)?;
            }

            WindowEvent::ThemeChanged(winit_theme) => {
                self.gui.set_theme_winit(winit_theme);
            }
            _ => (),
        }

        Ok(())
    }

    fn per_frame_processing(&mut self) -> Result<(), EngineError> {
        // make sure the render thread is active to receive the upcoming messages
        let thread_send_res = self
            .render_thread_channels
            .set_render_thread_command(RenderThreadCommand::Run);
        check_channel_updater_result(thread_send_res)?;

        // process recieved events for cursor state
        let cursor_event = self.cursor.process_frame();
        if let Some(cursor_icon) = self.cursor.get_cursor_icon() {
            self.gui.set_cursor_icon(cursor_icon);
        }

        // process gui inputs and update layout
        let update_gui_res = self.gui.update_gui(
            &self.window,
            self.camera,
            self.selected_object_id,
            self.selected_primitive_op_id,
            &mut self.object_collection,
        );
        let commands_from_gui = anyhow_unwrap(update_gui_res, "update gui");
        self.pending_commands.extend(commands_from_gui.into_iter());

        // process commands from gui
        self.execute_engine_commands();

        // update camera
        self.update_camera();
        let thread_send_res = self
            .render_thread_channels
            .update_camera(self.camera.clone());
        check_channel_updater_result(thread_send_res)?;

        // update object buffers todo better objects delta
        let objects_delta = self.object_collection.get_and_clear_objects_delta();
        if !objects_delta.is_empty() {
            let thread_send_res = self.render_thread_channels.update_objects(objects_delta);
            check_channel_sender_result(thread_send_res)?;
        }

        // update gui textures
        let textures_delta = self.gui.get_and_clear_textures_delta();
        if !textures_delta.is_empty() {
            let thread_send_res = self
                .render_thread_channels
                .update_gui_textures(textures_delta);
            check_channel_sender_result(thread_send_res)?;
        }

        // update gui primitives
        let gui_primitives = self.gui.mesh_primitives().clone();
        if !gui_primitives.is_empty() {
            let thread_send_res = self
                .render_thread_channels
                .set_gui_primitives(gui_primitives);
            check_channel_updater_result(thread_send_res)?;
        }

        // if render clicked, send request to find out which element on scene it is
        if let CursorEvent::LeftClickInPlace = cursor_event {
            self.submit_request_for_element_id_at_point()?;
        }

        // receive clicked element response
        self.receive_and_select_element_id_at_point();

        let latest_render_frame_timestamp = self
            .render_thread_channels
            .get_latest_render_frame_timestamp();

        self.main_thread_frame_number += 1;

        Ok(())
    }

    fn update_window_inner_size(
        &mut self,
        new_inner_size: winit::dpi::PhysicalSize<u32>,
    ) -> Result<(), EngineError> {
        self.camera.set_aspect_ratio(new_inner_size.into());
        let thread_send_res = self.render_thread_channels.set_window_just_resized_flag();

        check_channel_updater_result(thread_send_res)
    }

    fn set_scale_factor(&mut self, scale_factor: f64) -> Result<(), EngineError> {
        self.scale_factor = scale_factor;
        self.gui.set_scale_factor(self.scale_factor as f32);
        let thread_send_res = self
            .render_thread_channels
            .set_scale_factor(scale_factor as f32);

        check_channel_updater_result(thread_send_res)
    }

    fn update_camera(&mut self) {
        // if no primitive selected use arcball mode
        if self.gui.selected_object_id().is_none() {
            self.camera.unset_lock_on_target();
        }

        // left mouse button dragging changes camera orientation
        if self.cursor.which_dragging() == Some(MouseButton::Left) {
            self.camera
                .rotate_from_cursor_delta(self.cursor.position_frame_change());
        }

        // zoom in/out logic
        let scroll_delta = self.cursor.get_and_clear_scroll_delta();
        self.camera.scroll_zoom(scroll_delta.y);
    }

    fn submit_request_for_element_id_at_point(&mut self) -> Result<(), EngineError> {
        if let Some(cursor_screen_coordinates_dvec2) = self.cursor.position() {
            let cursor_screen_coordinates = cursor_screen_coordinates_dvec2.as_vec2().to_array();

            // send request
            let thread_send_res = self
                .render_thread_channels
                .request_element_id_at_screen_coordinate(cursor_screen_coordinates);
            check_channel_updater_result(thread_send_res)?;
        }
        Ok(())
    }

    fn receive_and_select_element_id_at_point(&mut self) {
        if let Some(element_at_point) = self
            .render_thread_channels
            .receive_element_id_at_screen_coordinate()
        {
            debug!("element clicked = {:?}", element_at_point);
            match element_at_point {
                ElementAtPoint::Background => self.background_clicked(),
                ElementAtPoint::Object {
                    object_id,
                    primitive_op_index,
                } => self.object_clicked(object_id, primitive_op_index),
            }
        }
    }

    fn stop_render_thread(&self) {
        debug!("sending quit command to render thread...");
        let _render_thread_send_res = self
            .render_thread_channels
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

// ~~ Misc UI Logic ~~

impl EngineInstance {
    fn background_clicked(&mut self) {
        self.deselect_primitive_op();
        self.gui.deselect_primitive_op();
    }

    fn object_clicked(&mut self, object_id: ObjectId, primitive_op_index: usize) {
        self.select_object_and_primitive_op(object_id, primitive_op_index)
    }
}

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

/// If `thread_send_res` is an error, returns `EngineError::RenderThreadClosedPrematurely`.
/// Otherwise returns `Ok`.
fn check_channel_updater_result<T>(
    thread_send_res: Result<(), NoReceiverError<T>>,
) -> Result<(), EngineError> {
    if let Err(e) = thread_send_res {
        warn!("render thread receiver dropped prematurely ({})", e);
        return Err(EngineError::RenderThreadClosedPrematurely);
    }
    Ok(())
}

/// If `thread_send_res` is an error, returns `EngineError::RenderThreadClosedPrematurely`.
/// Otherwise returns `Ok`.
fn check_channel_sender_result<T>(
    thread_send_res: Result<(), SendError<T>>,
) -> Result<(), EngineError> {
    if let Err(e) = thread_send_res {
        warn!("render thread receiver dropped prematurely ({})", e);
        return Err(EngineError::RenderThreadClosedPrematurely);
    }
    Ok(())
}

// ~~ Testing ~~

fn object_testing(object_collection: &mut ObjectCollection, renderer: &mut RenderManager) {
    let sphere = Sphere::new(Vec3::new(0., 0., 0.), Quat::IDENTITY, 0.5);
    let cube = Cube::new(
        Vec3::new(-0.2, 0.2, 0.),
        Quat::IDENTITY,
        glam::Vec3::splat(0.8),
    );
    let another_sphere = Sphere::new(Vec3::new(0.2, -0.2, 0.), Quat::IDENTITY, 0.83);

    let (object_id, object) =
        object_collection.new_object("Bruh".to_string(), Vec3::new(-0.2, 0.2, 0.));
    object.push_op(Operation::Union, Primitive::Cube(cube));
    object.push_op(Operation::Union, Primitive::Sphere(sphere.clone()));
    object.push_op(Operation::Intersection, Primitive::Sphere(another_sphere));
    let _ = object_collection.mark_object_for_data_update(object_id);

    let (another_object_id, another_object) =
        object_collection.new_object("Another Bruh".to_string(), Vec3::new(0.2, -0.2, 0.));
    another_object.push_op(Operation::Union, Primitive::Sphere(sphere));
    another_object.push_op(Operation::Union, Primitive::Null(NullPrimitive::new()));
    let _ = object_collection.mark_object_for_data_update(another_object_id);

    let objects_delta = object_collection.get_and_clear_objects_delta();
    let update_objects_res = renderer.update_objects(objects_delta);
    anyhow_unwrap(update_objects_res, "update object buffers");
}