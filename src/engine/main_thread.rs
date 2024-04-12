use crate::config;
use anyhow::Context;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::{
    sync::{
        mpsc::{self, TryRecvError},
        Arc,
    },
    thread,
};
use winit::{
    event::Event,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

use super::engine_controller::{EngineCommand, EngineController};

pub fn start_main_thread() -> anyhow::Result<()> {
    let event_loop = EventLoop::new().context("creating os event loop")?;

    let window = create_window(&event_loop)?;

    let (engine_command_rx, engine_command_tx) = single_value_channel::channel::<EngineCommand>();
    let (window_event_tx, window_event_rx) = mpsc::channel::<Event<()>>();

    let main_thread_channels = MainThreadChannels {
        engine_command_rx,
        window_event_rx,
    };

    let _ = engine_command_tx.update(Some(EngineCommand::Run));
    let engine_thread_handle = thread::spawn(move || {
        let mut engine_controller = EngineController::new(window, main_thread_channels)?;

        engine_controller.run()?;

        Ok::<(), anyhow::Error>(())
    });

    event_loop
        .run(move |event, event_loop_window_target| {
            // send os event to engine thread
            let send_res = window_event_tx.send(event);

            // handle shutdown
            if let Err(_e) = send_res {
                info!("engine thread disconnected. stopping main thread...");
                event_loop_window_target.exit();
            }
        })
        .context("OS event loop")?;

    // check reason for engine thread closure
    let engine_thread_join_res = engine_thread_handle.join();
    if let Err(engine_panic_param) = &engine_thread_join_res {
        error!("panic on engine thread! panic params:");
        error!("{:?}", engine_panic_param);
    }
    if let Ok(engine_thread_res) = engine_thread_join_res {
        match engine_thread_res {
            Ok(()) => info!("engine thread shut down cleanly."),
            Err(engine_thread_err) => error!(
                "engine thread shut down due to error: {}",
                engine_thread_err
            ),
        }
    }

    Ok(())
}

fn create_window(event_loop: &EventLoop<()>) -> anyhow::Result<Arc<Window>> {
    info!("creating main window...");
    let window_builder = WindowBuilder::new().with_title(config::ENGINE_NAME);
    let window = Arc::new(
        window_builder
            .build(event_loop)
            .context("instanciating initial os window")?,
    );
    Ok(window)
}

pub struct MainThreadChannels {
    /// FIFO queue
    pub engine_command_rx: single_value_channel::Receiver<Option<EngineCommand>>,
    pub window_event_rx: mpsc::Receiver<Event<()>>,
}

impl MainThreadChannels {
    /// Ordered by time received, i.e. first event in index 0
    pub fn get_events(&self) -> anyhow::Result<Vec<Event<()>>> {
        let mut events = Vec::<Event<()>>::new();
        loop {
            let recv_res = self.window_event_rx.try_recv();
            match recv_res {
                Ok(event) => events.push(event),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => anyhow::bail!("window thread disconnected"),
            };
        }
        Ok(events)
    }

    pub fn latest_command(&mut self) -> Option<EngineCommand> {
        *self.engine_command_rx.latest()
    }
}
