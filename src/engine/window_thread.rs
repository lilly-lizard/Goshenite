#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::{
    sync::{mpsc, Arc},
    thread::{self, JoinHandle},
};
use winit::{error::EventLoopError, event::Event, event_loop::EventLoop, window::WindowBuilder};

use crate::config;

pub fn start_window_thread() -> (JoinHandle<Result<(), EventLoopError>>, WindowThreadChannels) {
    let (window_event_tx, window_event_rx) = mpsc::channel::<Event<()>>();

    let window_thread_handle = thread::spawn(move || {
        let event_loop = EventLoop::new()?;

        let mut window_builder = WindowBuilder::new().with_title(config::ENGINE_NAME);
        let window = Arc::new(
            window_builder
                .build(&event_loop)
                .expect("failed to instanciate window due to os error"),
        );

        event_loop.run(move |event, event_loop_window_target| {
            window_event_tx.send(event);
        })
    });

    (
        window_thread_handle,
        WindowThreadChannels { window_event_rx },
    )
}

pub struct WindowThreadChannels {
    pub window_event_rx: mpsc::Receiver<Event<()>>,
}
