use crate::config;
use anyhow::Context;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::{
    error, fmt,
    sync::{
        mpsc::{self, TryRecvError},
        Arc,
    },
    thread::{self, JoinHandle},
};
use winit::{
    error::EventLoopError,
    event::Event,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

pub enum WindowThreadCommand {
    Exit,
}

pub fn start_window_thread() -> anyhow::Result<(
    Arc<Window>,
    JoinHandle<Result<(), WindowThreadError>>,
    WindowThreadChannels,
)> {
    let (window_tx, window_rx) = mpsc::channel::<Arc<Window>>();
    let (mut window_command_rx, window_command_tx) =
        single_value_channel::channel::<WindowThreadCommand>();
    let (window_event_tx, window_event_rx) = mpsc::channel::<Event<()>>();

    let window_thread_handle = thread::spawn(move || {
        let event_loop = EventLoop::new()?;

        info!("creating main window...");
        let window_builder = WindowBuilder::new().with_title(config::ENGINE_NAME);
        let window = Arc::new(
            window_builder
                .build(&event_loop)
                .expect("failed to instanciate window due to os error"),
        );

        debug!("sending window to main thread...");
        let window_send_res = window_tx.send(window);
        if let Err(_e) = window_send_res {
            error!(
                "window thread > receiver disconnected on main thread. stopping window thread..."
            );
            return Err(WindowThreadError::ReceiverDisconnected);
        }

        event_loop
            .run(move |event, event_loop_window_target| {
                // receive command from main thread
                if let Some(command) = window_command_rx.latest() {
                    match *command {
                        WindowThreadCommand::Exit => {
                            event_loop_window_target.exit();
                            return;
                        }
                    }
                }
                
                // send each os event back to the main thread
                let send_res = window_event_tx.send(event);
                if let Err(_e) = send_res {
                    error!("window thread > receiver disconnected on main thread. stopping window thread...");
                    event_loop_window_target.exit();
                    return;
                }
            })
            .map_err(WindowThreadError::from)
    });

    let window = window_rx
        .recv()
        .context("receiving window from window thread")?;

    Ok((
        window,
        window_thread_handle,
        WindowThreadChannels {
            window_command_tx,
            window_event_rx,
        },
    ))
}

pub struct WindowThreadChannels {
    /// FIFO queue
    pub window_command_tx: single_value_channel::Updater<Option<WindowThreadCommand>>,
    pub window_event_rx: mpsc::Receiver<Event<()>>,
}

impl WindowThreadChannels {
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
}

#[derive(Debug)]
pub enum WindowThreadError {
    ReceiverDisconnected,
    EventLoopError(EventLoopError),
}

impl From<EventLoopError> for WindowThreadError {
    fn from(e: EventLoopError) -> Self {
        Self::EventLoopError(e)
    }
}

impl fmt::Display for WindowThreadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReceiverDisconnected => write!(f, "receiver disconnected in parent thread"),
            Self::EventLoopError(e) => write!(f, "winit event loop error: {}", e),
        }
    }
}

impl error::Error for WindowThreadError {}
