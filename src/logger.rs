use std::fs::OpenOptions;
use std::io::Write;
use chrono::Utc;

pub struct Logger {
    pub debug_mode: bool,
    pub log_path:   String,
}

impl Logger {
    pub fn new(debug: bool) -> Self {
        Self {
            debug_mode: debug,
            log_path:   "ghostcoin.log".to_string(),
        }
    }

    fn write(&self, level: &str, msg: &str) {
        let line = format!("[{}] [{}] {}\n",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            level, msg);

        if self.debug_mode || level == "ERROR" {
            print!("{}", line);
        }

        if let Ok(mut f) = OpenOptions::new()
            .create(true).append(true).open(&self.log_path)
        {
            let _ = f.write_all(line.as_bytes());
        }
    }

    pub fn info(&self, msg: &str)  { self.write("INFO",  msg); }
    pub fn warn(&self, msg: &str)  { self.write("WARN",  msg); }
    pub fn error(&self, msg: &str) { self.write("ERROR", msg); }
    pub fn debug(&self, msg: &str) { if self.debug_mode { self.write("DEBUG", msg); } }
    pub fn tx(&self, msg: &str)    { self.write("TX",    msg); }
    pub fn mining(&self, msg: &str){ self.write("MINE",  msg); }
    pub fn peer(&self, msg: &str)  { self.write("PEER",  msg); }
}

// Logger global
use std::sync::OnceLock;
static LOGGER: OnceLock<Logger> = OnceLock::new();

pub fn init_logger(debug: bool) {
    let _ = LOGGER.set(Logger::new(debug));
}

pub fn log_info(msg: &str)   { if let Some(l) = LOGGER.get() { l.info(msg);   } }
pub fn log_warn(msg: &str)   { if let Some(l) = LOGGER.get() { l.warn(msg);   } }
pub fn log_error(msg: &str)  { if let Some(l) = LOGGER.get() { l.error(msg);  } }
pub fn log_debug(msg: &str)  { if let Some(l) = LOGGER.get() { l.debug(msg);  } }
pub fn log_tx(msg: &str)     { if let Some(l) = LOGGER.get() { l.tx(msg);     } }
pub fn log_mining(msg: &str) { if let Some(l) = LOGGER.get() { l.mining(msg); } }
pub fn log_peer(msg: &str)   { if let Some(l) = LOGGER.get() { l.peer(msg);   } }