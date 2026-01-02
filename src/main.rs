#![allow(dead_code)]
#![allow(unused_macros)]

mod config;
mod engine;
mod helper;
mod renderer;
mod user_interface;

use crate::engine::{
    engine_controller::{EngineCommand, EngineController},
    window_thread::{WindowThread, WindowThreadChannels},
};
use anyhow::Context;
use helper::logger::ConsoleLogger;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use std::{mem::ManuallyDrop, sync::mpsc, sync::Arc, thread};
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

    start_main_thread()
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

pub fn start_main_thread() -> anyhow::Result<()> {
    let event_loop = EventLoop::new().context("creating os event loop")?;

    let window = create_window(&event_loop)?;
    let primary_window_id = window.id();

    let (engine_command_rx, engine_command_tx) = single_value_channel::channel::<EngineCommand>();
    let (window_event_tx, window_event_rx) = mpsc::channel::<WindowEvent>();

    let main_thread_channels = WindowThreadChannels {
        engine_command_rx,
        window_event_rx,
    };

    // engine thread is separate from window event polling (so I'm not constricted by winit's loop structure which is subject to change)
    let _ = engine_command_tx.update(Some(EngineCommand::Run));
    let engine_thread_handle = thread::spawn(|| {
        info!("initializing engine instance");
        let mut engine_controller = EngineController::new(window, main_thread_channels)?;

        info!("starting engine loop");
        engine_controller.run()?;

        Ok::<(), anyhow::Error>(())
    });

    // main thread is responsible for recieving window events
    let mut window_thread = WindowThread {
        primary_window_id,
        engine_thread_handle: ManuallyDrop::new(engine_thread_handle),
        engine_command_tx,
        window_event_tx,
    };
    event_loop.run_app(&mut window_thread)?;

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
