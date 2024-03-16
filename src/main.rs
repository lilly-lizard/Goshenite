#![allow(dead_code)]
#![allow(unused_macros)]

mod config;
mod engine;
mod helper;
mod renderer;
mod user_interface;

use engine::engine_controller::EngineController;
use helper::logger::ConsoleLogger;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use winit::event_loop::EventLoop;

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

fn main() -> Result<(), impl std::error::Error> {
    println!("{}", SPLASH);

    init_logger();

    info!(
        "if debugging, set environment variable `RUST_BACKTRACE=1` to see anyhow error backtrace"
    );

    let event_loop = EventLoop::new()?;

    // init engine
    let mut engine_instance = EngineController::new(&event_loop);

    // start engine
    event_loop.run(move |event, event_loop_window_target| {
        engine_instance.control_flow(event, event_loop_window_target)
    })
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
