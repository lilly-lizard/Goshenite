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
                Level::Error => ("[E]", Color::Red),
                Level::Warn => ("[W]", Color::Yellow),
                Level::Info => ("[I]", Color::Blue),
                Level::Debug => ("[D]", Color::Magenta),
                Level::Trace => ("[T]", Color::White),
            };
            println!(
                "{} {} -> {}",
                level.color(color),
                record
                    .module_path()
                    .unwrap_or("(unknown module)")
                    .color(color),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}
