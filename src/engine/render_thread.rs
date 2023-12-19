use crate::{
    helper::anyhow_panic::anyhow_unwrap,
    renderer::{
        config_renderer::RenderOverlayOptions, element_id_reader::ElementAtPoint,
        render_manager::RenderManager,
    },
    user_interface::camera::Camera,
};
use egui::{ClippedPrimitive, TexturesDelta};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use single_value_channel::NoReceiverError;
use std::{
    mem,
    sync::mpsc::{self, SendError},
    thread::{self, JoinHandle},
    time::Instant,
};

use super::object::objects_delta::ObjectsDelta;

#[derive(Clone, Copy)]
pub enum RenderThreadCommand {
    DoNothing,
    Run(RenderOverlayOptions),
    Quit,
}

#[derive(Copy, Clone)]
pub struct RenderFrameTimestamp {
    pub frame_num: u64,
    pub timestamp: Instant,
}

impl RenderFrameTimestamp {
    pub fn start() -> Self {
        Self {
            frame_num: 0,
            timestamp: Instant::now(),
        }
    }

    pub fn incriment(previous_frame_number: u64) -> Self {
        Self {
            frame_num: previous_frame_number + 1,
            timestamp: Instant::now(),
        }
    }
}

pub fn start_render_thread(mut renderer: RenderManager) -> (JoinHandle<()>, RenderThreadChannels) {
    let (mut render_command_rx, render_command_tx) = single_value_channel::channel_starting_with::<
        RenderThreadCommand,
    >(RenderThreadCommand::DoNothing);

    let (mut window_resize_flag_rx, window_resize_flag_tx) =
        single_value_channel::channel::<bool>();

    let (mut scale_factor_rx, scale_factor_tx) = single_value_channel::channel::<f32>();

    let (mut camera_rx, camera_tx) = single_value_channel::channel::<Camera>();

    let (objects_delta_tx, objects_delta_rx) = mpsc::channel::<ObjectsDelta>();

    let (textures_delta_tx, textures_delta_rx) = mpsc::channel::<Vec<TexturesDelta>>();

    let (mut gui_primitives_rx, gui_primitives_tx) =
        single_value_channel::channel::<Vec<ClippedPrimitive>>();

    let initial_render_frame_timestamp = RenderFrameTimestamp::start();
    let (frame_timestamp_rx, frame_timestamp_tx) = single_value_channel::channel();

    let (mut element_id_coordinate_rx, element_id_coordinate_tx) =
        single_value_channel::channel::<[f32; 2]>();
    let (element_id_rx, element_id_tx) = single_value_channel::channel::<ElementAtPoint>();

    // render thread loop
    let render_thread_handle = thread::spawn(move || {
        let mut frame_timestamp = initial_render_frame_timestamp;

        loop {
            // receive and process command from main thread

            let render_command = render_command_rx.latest();
            let render_options = match render_command {
                RenderThreadCommand::Quit => break,
                RenderThreadCommand::DoNothing => continue,
                RenderThreadCommand::Run(options) => *options,
            };

            // check for state updates

            if let Some(window_resized_flag) = mem::take(window_resize_flag_rx.latest_mut()) {
                if window_resized_flag {
                    renderer.set_window_just_resized_flag();
                }
            }

            if let Some(scale_factor) = mem::take(scale_factor_rx.latest_mut()) {
                renderer.set_scale_factor(scale_factor);
            }

            if let Some(camera) = mem::take(camera_rx.latest_mut()) {
                let update_camera_res = renderer.update_camera(&camera);
                anyhow_unwrap(update_camera_res, "update camera buffer");
            }

            // the main thread may have sent multiple objects delta packages since we last checked...
            loop {
                match objects_delta_rx.try_recv() {
                    Ok(objects_delta) => {
                        let update_objects_res = renderer.update_objects(objects_delta);
                        anyhow_unwrap(update_objects_res, "update object buffers");
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        error!("render thread > textures delta sender disconnected! stopping render thread...");
                        break;
                    }
                }
            }

            // the main thread may have sent multiple texture delta packages since we last checked...
            loop {
                match textures_delta_rx.try_recv() {
                    Ok(textures_delta) => {
                        let update_textures_res = renderer.update_gui_textures(textures_delta);
                        anyhow_unwrap(update_textures_res, "update gui textures");
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        error!("render thread > textures delta sender disconnected! stopping render thread...");
                        break;
                    }
                }
            }

            if let Some(gui_primitives) = mem::take(gui_primitives_rx.latest_mut()) {
                renderer.set_gui_primitives(gui_primitives);
            }

            // request data from previous render. this is done just before next-frame submission to
            // minimize fence stalling

            if let Some(screen_coordinate) = mem::take(element_id_coordinate_rx.latest_mut()) {
                let get_element_res = renderer.get_element_at_screen_coordinate(screen_coordinate);
                let element_id =
                    anyhow_unwrap(get_element_res, "getting element id at screen cordinate");

                if let Err(NoReceiverError(_)) = element_id_tx.update(element_id) {
                    error!("render thread > element id receiver disconnected! stopping render thread...");
                    break;
                }
            }

            // submit frame rendering commands

            let render_frame_res = renderer.render_frame(render_options);
            anyhow_unwrap(render_frame_res, "render frame");

            // send new frame timestamp

            frame_timestamp = RenderFrameTimestamp::incriment(frame_timestamp.frame_num);
            if let Err(NoReceiverError(_)) = frame_timestamp_tx.update(Some(frame_timestamp)) {
                error!("render thread > frame timestamp receiver disconnected! stopping render thread...");
                break;
            }
        }
    });

    (
        render_thread_handle,
        RenderThreadChannels {
            render_command_tx,

            window_resize_flag_tx,
            scale_factor_tx,

            camera_tx,
            objects_delta_tx,
            textures_delta_tx,
            gui_primitives_tx,

            element_id_coordinate_tx,
            element_id_rx,

            frame_timestamp_rx,
        },
    )
}

/// Render thread channel handles for the main thread to send/receive data
pub struct RenderThreadChannels {
    pub render_command_tx: single_value_channel::Updater<RenderThreadCommand>,

    pub window_resize_flag_tx: single_value_channel::Updater<Option<bool>>,
    pub scale_factor_tx: single_value_channel::Updater<Option<f32>>,

    pub camera_tx: single_value_channel::Updater<Option<Camera>>,
    pub objects_delta_tx: mpsc::Sender<ObjectsDelta>,
    pub textures_delta_tx: mpsc::Sender<Vec<TexturesDelta>>,
    pub gui_primitives_tx: single_value_channel::Updater<Option<Vec<ClippedPrimitive>>>,

    pub element_id_coordinate_tx: single_value_channel::Updater<Option<[f32; 2]>>,
    pub element_id_rx: single_value_channel::Receiver<Option<ElementAtPoint>>,

    pub frame_timestamp_rx: single_value_channel::Receiver<Option<RenderFrameTimestamp>>,
}

impl RenderThreadChannels {
    pub fn set_render_thread_command(
        &self,
        command: RenderThreadCommand,
    ) -> Result<(), NoReceiverError<RenderThreadCommand>> {
        self.render_command_tx.update(command)
    }

    pub fn set_window_just_resized_flag(&self) -> Result<(), NoReceiverError<Option<bool>>> {
        self.window_resize_flag_tx.update(Some(true))
    }

    pub fn set_scale_factor(&self, scale_factor: f32) -> Result<(), NoReceiverError<Option<f32>>> {
        self.scale_factor_tx.update(Some(scale_factor))
    }

    pub fn update_camera(&self, camera: Camera) -> Result<(), NoReceiverError<Option<Camera>>> {
        self.camera_tx.update(Some(camera))
    }

    pub fn update_objects(
        &self,
        objects_delta: ObjectsDelta,
    ) -> Result<(), SendError<ObjectsDelta>> {
        self.objects_delta_tx.send(objects_delta)
    }

    pub fn update_gui_textures(
        &self,
        textures_delta: Vec<TexturesDelta>,
    ) -> Result<(), SendError<Vec<TexturesDelta>>> {
        self.textures_delta_tx.send(textures_delta)
    }

    pub fn set_gui_primitives(
        &self,
        gui_primitives: Vec<ClippedPrimitive>,
    ) -> Result<(), NoReceiverError<Option<Vec<ClippedPrimitive>>>> {
        self.gui_primitives_tx.update(Some(gui_primitives))
    }

    pub fn request_element_id_at_screen_coordinate(
        &self,
        screen_coordinates: [f32; 2],
    ) -> Result<(), NoReceiverError<Option<[f32; 2]>>> {
        self.element_id_coordinate_tx
            .update(Some(screen_coordinates))
    }

    pub fn receive_element_id_at_screen_coordinate(&mut self) -> Option<ElementAtPoint> {
        mem::take(self.element_id_rx.latest_mut())
    }

    pub fn get_latest_render_frame_timestamp(&mut self) -> Option<RenderFrameTimestamp> {
        mem::take(self.frame_timestamp_rx.latest_mut())
    }
}
