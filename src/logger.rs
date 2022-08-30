use colored::Colorize;
use log::{Level, Metadata, Record};

pub struct ConsoleLogger;

impl log::Log for ConsoleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let level = match record.level() {
                Level::Error => "ERROR".red(),
                Level::Warn => "WARN ".yellow(),
                Level::Info => "INFO ".blue(),
                Level::Debug => "DEBUG".magenta(),
                Level::Trace => "TRACE".normal(),
            };
            println!("[Goshenite] {}: {}", level, record.args());
        }
    }

    fn flush(&self) {}
}
