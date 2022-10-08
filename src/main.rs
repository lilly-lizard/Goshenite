mod camera;
mod config;
mod cursor_state;
mod engine;
mod gui;
mod helper;
mod primitives;
mod renderer;
mod shaders;

use engine::Engine;
use helper::logger::ConsoleLogger;
use log::info;
use winit::{event_loop::EventLoop, platform::run_return::EventLoopExtRunReturn};

/// spooky symbols, what could it mean...
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

fn main() {
    println!("{}", SPLASH);

    // init logger
    static CONSOLE_LOGGER: ConsoleLogger = ConsoleLogger;
    if let Err(e) = log::set_logger(&CONSOLE_LOGGER) {
        println!("Goshenite ERROR - Failed to initialize logger: {:?}", e);
    };
    log::set_max_level(config::DEFAULT_LOG_LEVEL);

    info!(
        "if debugging, set environment variable `RUST_BACKTRACE=1` to see anyhow error backtrace"
    );

    // init engine
    let mut event_loop = EventLoop::new();
    let mut engine_instance = Engine::new(&event_loop);

    // start engine
    event_loop
        .run_return(|event, _, control_flow| engine_instance.control_flow(event, control_flow));
}
