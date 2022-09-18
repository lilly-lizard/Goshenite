use colored::{Color, Colorize};
use log::{Level, Metadata, Record};

/// A simple [`log`] implimentation which I found easier to configure than using something
/// like `env_logger` (and less to compile too)
pub struct ConsoleLogger;

impl log::Log for ConsoleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let (level, color) = match record.level() {
                Level::Error => ("[E]", Color::Red),
                Level::Warn => ("[W]", Color::Yellow),
                Level::Info => ("[I]", Color::Cyan),
                Level::Debug => ("[D]", Color::Magenta),
                Level::Trace => ("[T]", Color::Blue),
            };
            println!(
                "{} {} {} {}",
                level.color(color),
                record
                    .module_path()
                    .unwrap_or("(unknown module)")
                    .color(color),
                ">".color(color),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}
