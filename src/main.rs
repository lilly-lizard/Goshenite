#![allow(unused_macros)]

mod config;
mod engine;
mod helper;
mod renderer;
mod user_interface;

use crate::engine::{
    engine::{Engine, EngineCommand},
    window_thread::{WindowThread, WindowThreadChannels},
};
use anyhow::Context;
use helper::logger::ConsoleLogger;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::{sync::mpsc, sync::Arc, thread};
use winit::{
    event::WindowEvent,
    event_loop::EventLoop,
    window::{Window, WindowAttributes},
};

const SPLASH: &str = "
     ___        ___        ___        ___        ___        ___        ___       ___        ___
    /\\  \\      /\\  \\      /\\  \\      /\\__\\      /\\  \\      /\\__\\      /\\  \\     /\\  \\      /\\  \\
   /  \\  \\    /  \\  \\    /  \\  \\    / /  /     /  \\  \\    / /  /      \\ \\  \\    \\ \\  \\    /  \\  \\
  / /\\ \\  \\  / /\\ \\  \\  / /\\ \\  \\  / /__/     / /\\ \\  \\  / /  /        \\ \\  \\    \\ \\  \\  / /\\ \\  \\
 / /  \\ \\  \\/ /  \\ \\  \\_\\ \\ \\ \\  \\/  \\  \\ ___/  \\ \\ \\  \\/ /__/_____ __ /  \\  \\   /  \\  \\/  \\ \\ \\  \\
/ /__/ \\ \\__\\/__/ \\ \\__\\ \\ \\ \\ \\__\\/\\ \\  /\\__\\/\\ \\ \\ \\__\\ _____ \\__\\  / /\\ \\__\\ / /\\ \\__\\/\\ \\ \\ \\__\\
\\ \\  /\\ \\/__/\\  \\ / /  /\\ \\ \\ \\/__/__\\ \\/ /  /\\ \\ \\ \\/__/__/  / /  /\\/ /  \\/__// /  \\/__/\\ \\ \\ \\/__/
 \\ \\ \\ \\__\\ \\ \\  / /  /\\ \\ \\ \\__\\     \\  /  /\\ \\ \\ \\__\\      / /  /\\  /__/    / /  /    \\ \\ \\ \\__\\
  \\ \\/ /  /  \\ \\/ /  /  \\ \\/ /  /     / /  /  \\ \\ \\/__/     / /  /  \\ \\  \\    \\/__/      \\ \\ \\/__/
   \\  /  /    \\  /  /    \\  /  /     / /  /    \\ \\__\\      / /  /    \\ \\__\\               \\ \\__\\
    \\/__/      \\/__/      \\/__/      \\/__/      \\/__/      \\/__/      \\/__/                \\/__/
";

static CONSOLE_LOGGER: ConsoleLogger = ConsoleLogger;

fn main() -> Result<(), anyhow::Error> {
    println!("{}", SPLASH);
    init_logger();
    info!(
        "if debugging, set environment variable `RUST_BACKTRACE=1` to see anyhow error backtrace"
    );
    start_engine()
}

fn init_logger() {
    let set_logger_res = log::set_logger(&CONSOLE_LOGGER);
    if let Err(e) = set_logger_res {
        println!("Goshenite ERROR - Failed to initialize logger: {:?}", e);
    };

    log::set_max_level(config::DEFAULT_LOG_LEVEL);

    // otherwise colors wont work in cmd https://github.com/mackwic/colored/issues/59#issuecomment-954355180
    #[cfg(all(feature = "colored-term", target_os = "windows"))]
    colored::control::set_virtual_terminal(true).expect("always Ok");
}

pub fn start_engine() -> anyhow::Result<()> {
    let event_loop = EventLoop::new().context("creating os event loop")?;

    let window = create_window(&event_loop)?;
    let primary_window_id = window.id();

    let (engine_command_rx, engine_command_tx) = single_value_channel::channel::<EngineCommand>();
    let (window_event_tx, window_event_rx) = mpsc::channel::<WindowEvent>();
    // ensures that the renderer shuts down before the OS window objects are destroyed
    let (engine_closed_flag_tx, engine_closed_flag_rx) = mpsc::channel::<bool>();

    let window_thread_channels = WindowThreadChannels {
        engine_command_rx,
        window_event_rx,
    };

    // start separate thread that runs the engine
    // engine thread is separate from window event polling (so I'm not constricted by winit's loop structure which is subject to change)
    let _ = engine_command_tx.update(Some(EngineCommand::Run));
    let engine_thread_handle = thread::spawn(move || {
        let engine_run_res = {
            info!("initializing engine instance");
            let mut engine_controller = Engine::new(window, window_thread_channels)?;

            info!("starting engine loop");
            engine_controller.run()
        };

        // tell the window thread that the engine controller is dropped so it is safe to start detroying the OS window objects
        if let Err(e) = engine_closed_flag_tx.send(true) {
            warn!(
                "error while sending engine closed status to window thread: {:?}",
                e
            );
        }
        engine_run_res
    });

    // main thread is responsible for recieving window events
    let mut window_thread = WindowThread {
        primary_window_id,
        engine_command_tx,
        window_event_tx,
        engine_closed_flag_rx,
    };
    event_loop.run_app(&mut window_thread)?;

    wait_for_engine_thread_closure(engine_thread_handle)?;

    debug!("main thread ended sucessfully...");
    Ok(())
}

fn create_window(event_loop: &EventLoop<()>) -> anyhow::Result<Arc<Window>> {
    info!("creating main window...");
    let window_attributes = WindowAttributes::default().with_title(config::ENGINE_NAME);
    let window = event_loop
        .create_window(window_attributes)
        .context("instanciating initial os window")?;
    Ok(Arc::new(window))
}

fn wait_for_engine_thread_closure(
    engine_thread_handle: thread::JoinHandle<Result<(), anyhow::Error>>,
) -> anyhow::Result<()> {
    // check reason for engine thread closure
    let engine_thread_join_res = engine_thread_handle.join();
    match engine_thread_join_res {
        Ok(engine_thread_res) => return engine_thread_res,
        Err(engine_panic_param) => {
            error!("panic on engine thread! panic params:");
            error!("{:?}", engine_panic_param);
            anyhow::bail!("{:?}", engine_panic_param);
        }
    }
}
