#[cfg(feature = "colored-term")]
use colored::{Color, ColoredString, Colorize};
use log::{Level, Metadata, Record};

/// A simple [`log`] implimentation which I found easier to configure than using something
/// like `env_logger` (and less to compile too)
pub struct ConsoleLogger;

impl log::Log for ConsoleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
    }

    #[cfg(feature = "colored-term")]
    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            // level color
            let color = match record.level() {
                Level::Error => Color::BrightRed,
                Level::Warn => Color::Yellow,
                Level::Info => Color::Cyan,
                Level::Debug => Color::Magenta,
                Level::Trace => Color::Blue,
            };
            // log message
            let args = format!("{}", record.args());
            let args = if record.level() == Level::Error {
                // only color error message to make them stand out
                args.color(Color::Red)
            } else {
                ColoredString::from(args.as_str())
            };
            let module_path = record
                .module_path()
                .unwrap_or("(unknown module)")
                .color(color);
            if module_path.contains("winit") {
                // certain crates bombard the log output every frame
                return;
            }
            println!(
                "{} {} {} {}",
                level_str(record.level()).color(color),
                module_path,
                ">".color(color),
                args,
            );
        }
    }

    #[cfg(not(feature = "colored-term"))]
    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!(
                "{} {} {} {}",
                level_str(record.level()),
                record.module_path().unwrap_or("(unknown module)"),
                ">",
                record.args(),
            );
        }
    }

    fn flush(&self) {}
}

fn level_str(level: Level) -> &'static str {
    match level {
        Level::Error => "[E]",
        Level::Warn => "[W]",
        Level::Info => "[I]",
        Level::Debug => "[D]",
        Level::Trace => "[T]",
    }
}
