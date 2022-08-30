use colored::{Color, Colorize};
use log::{Level, Metadata, Record};

pub struct ConsoleLogger;

impl log::Log for ConsoleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let (level, color) = match record.level() {
                Level::Error => ("[ERROR]", Color::Red),
                Level::Warn => ("[WARN ]", Color::Yellow),
                Level::Info => ("[INFO ]", Color::Blue),
                Level::Debug => ("[DEBUG]", Color::Magenta),
                Level::Trace => ("[TRACE]", Color::White),
            };
            println!(
                "{} {} - {}",
                level.color(color),
                record.module_path().unwrap_or("...").color(color),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}
