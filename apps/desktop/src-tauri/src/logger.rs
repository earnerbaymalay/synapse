use log::{Level, Log, Metadata, Record};
use std::sync::Mutex;

pub struct SimpleLogger;

impl Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!("[{}] {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

pub fn init_logger() {
    let logger = SimpleLogger;
    let _ = log::set_boxed_logger(Box::new(logger));
    log::set_max_level(log::LevelFilter::Info);
}
