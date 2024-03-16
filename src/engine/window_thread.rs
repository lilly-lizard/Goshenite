#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::{
    sync::mpsc,
    thread::{self, JoinHandle},
};
use winit::{
    event::{Event, StartCause, WindowEvent},
    event_loop::EventLoop,
};

pub fn start_window_thread(event_loop: EventLoop<()>) -> (JoinHandle<()>, WindowThreadChannels) {
    let (window_event_tx, window_event_rx) = mpsc::channel::<Vec<Windowevent>>();

    let window_thread_handle = thread::spawn(move || {
        event_loop.run(move |event, event_loop_window_target| {
            match event {
                // exit the event loop and close application
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    info!("close requested by window");

                    // quit
                    self.stop_render_thread();
                    event_loop_window_target.exit();
                }

                // process window events and update state
                Event::WindowEvent { event, .. } => {
                    let process_input_res = self.process_window_event(event);

                    if let Err(e) = process_input_res {
                        error!("error while processing input: {}", e);

                        // quit
                        self.stop_render_thread();
                        event_loop_window_target.exit();
                    }
                }

                _ => (),
            }
        })
    });

    (
        window_thread_handle,
        WindowThreadChannels { window_event_rx },
    )
}

pub struct WindowThreadChannels {
    pub window_event_rx: mpsc::Receiver<Vec<WindowEvent>>,
}
