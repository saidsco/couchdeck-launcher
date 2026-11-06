//! Tiny dependency-free file logger.
//!
//! Logs are appended to `$XDG_STATE_HOME/couchdeck/couchdeck.log` and also
//! mirrored to stderr so launching from a terminal remains useful.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

static LOG_FILE: OnceLock<Mutex<Option<File>>> = OnceLock::new();

pub fn init(path: &Path) {
    let file = path.parent().and_then(|parent| {
        fs::create_dir_all(parent).ok()?;
        OpenOptions::new().create(true).append(true).open(path).ok()
    });
    let _ = LOG_FILE.set(Mutex::new(file));
    log_info("couchdeck", format!("Logdatei: {}", path.display()));
}

fn write(level: &str, target: &str, message: &str) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let line = format!("[{now}] {level:<5} {target}: {message}\n");
    eprint!("{line}");
    if let Some(lock) = LOG_FILE.get() {
        if let Ok(mut guard) = lock.lock() {
            if let Some(file) = guard.as_mut() {
                let _ = file.write_all(line.as_bytes());
                let _ = file.flush();
            }
        }
    }
}

pub fn log_info(target: &str, message: impl AsRef<str>) {
    write("INFO", target, message.as_ref());
}

pub fn log_warn(target: &str, message: impl AsRef<str>) {
    write("WARN", target, message.as_ref());
}

pub fn log_error(target: &str, message: impl AsRef<str>) {
    write("ERROR", target, message.as_ref());
}

pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log_error("panic", format!("{info}"));
        previous(info);
    }));
}
