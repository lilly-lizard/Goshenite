use super::object::objects_delta::{push_object_delta, ObjectsDelta};
use crate::{
    helper::anyhow_panic::anyhow_unwrap,
    renderer::{
        config_renderer::RenderOptions, element_id_reader::ElementAtPoint,
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

#[derive(Copy, Clone)]
pub struct FrameTimestamp {
    pub frame_num: u64,
    pub timestamp: Instant,
}

impl FrameTimestamp {
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

fn render_loop(
    mut renderer: RenderManager,
    initial_render_frame_timestamp: RenderFrameTimestamp,
    mut render_command_rx: single_value_channel::Receiver<RenderThreadCommand>,
    mut window_resize_flag_rx: single_value_channel::Receiver<Option<bool>>,
    mut scale_factor_rx: single_value_channel::Receiver<Option<f32>>,
    mut camera_rx: single_value_channel::Receiver<Option<Camera>>,
    objects_delta_rx: mpsc::Receiver<
        std::collections::HashMap<
            super::object::object::ObjectId,
            super::object::objects_delta::ObjectDeltaOperation,
            ahash::RandomState,
        >,
    >,
    textures_delta_rx: mpsc::Receiver<Vec<TexturesDelta>>,
    mut gui_primitives_rx: single_value_channel::Receiver<Option<Vec<ClippedPrimitive>>>,
    mut element_id_coordinate_rx: single_value_channel::Receiver<Option<[f32; 2]>>,
    element_id_tx: single_value_channel::Updater<Option<ElementAtPoint>>,
    frame_timestamp_tx: single_value_channel::Updater<Option<RenderFrameTimestamp>>,
) {
    let mut frame_timestamp = initial_render_frame_timestamp;

    'render_loop: loop {
        // request data from previous render. this is done just before next-frame submission to
        // minimize fence stalling

        if let Some(screen_coordinate) = mem::take(element_id_coordinate_rx.latest_mut()) {
            let get_element_res = renderer.get_element_at_screen_coordinate(screen_coordinate);
            let element_id =
                anyhow_unwrap(get_element_res, "getting element id at screen cordinate");

            if let Err(NoReceiverError(_)) = element_id_tx.update(element_id) {
                error!(
                    "render thread > element id receiver disconnected! stopping render thread..."
                );
                break 'render_loop;
            }
        }

        // submit frame rendering commands

        let render_frame_res = renderer.render_frame(render_options);
        anyhow_unwrap(render_frame_res, "render frame");

        // send new frame timestamp

        frame_timestamp = RenderFrameTimestamp::incriment(frame_timestamp.frame_num);
        if let Err(NoReceiverError(_)) = frame_timestamp_tx.update(Some(frame_timestamp)) {
            error!(
                "render thread > frame timestamp receiver disconnected! stopping render thread..."
            );
            break 'render_loop;
        }
    }
}
