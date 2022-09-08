mod camera;
mod config;
mod cursor_state;
mod engine;
mod helper;
mod renderer;
mod shaders;

// todo propogate errors WHEREVER possible

use engine::EngineEntry;
use helper::logger::ConsoleLogger;
use log::LevelFilter;

/// spooky symbols, what could it mean...
const SPLASH: &str = "
     ___        ___        ___        ___        ___        ___        ___       ___        ___     
    /\\  \\      /\\  \\      /\\  \\      /\\__\\      /\\  \\      /\\__\\      /\\  \\     /\\  \\      /\\  \\    
   /  \\  \\    /  \\  \\    /  \\  \\    / /  /     /  \\  \\    / /  /      \\ \\  \\    \\ \\  \\    /  \\  \\   
  / /\\ \\  \\  / /\\ \\  \\  / /\\ \\  \\  / /__/     / /\\ \\  \\  / /  /        \\ \\  \\    \\ \\  \\  / /\\ \\  \\  
 / /  \\ \\  \\/ /  \\ \\  \\ \\ \\ \\ \\  \\/  \\  \\ ___/  \\ \\ \\  \\/ /__/_____ __ /  \\  \\   /  \\  \\/  \\ \\ \\  \\ 
/ /__/ \\ \\__\\/__/ \\ \\__\\ \\ \\ \\ \\__\\/\\ \\  /\\__\\/\\ \\ \\ \\__\\ _____ \\__\\  / /\\ \\__\\ / /\\ \\  \\/\\ \\ \\ \\__\\
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
    log::set_max_level(LevelFilter::Debug);

    // init engine
    let mut engine_entry = EngineEntry::init();

    // start engine
    engine_entry.start();
}
