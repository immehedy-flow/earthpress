//! File logging, ported from the Electron build.
//!
//! Entries land in `~/.earthpress/earthpress.log` as
//! `[timestamp] [LEVEL] message {json}`. Every failure here is swallowed: a
//! missing log file must never break a conversion.

use chrono::Utc;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;

static LOG_FILE: OnceLock<Option<PathBuf>> = OnceLock::new();

fn log_file() -> Option<&'static PathBuf> {
    LOG_FILE
        .get_or_init(|| {
            let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))?;
            let dir = PathBuf::from(home).join(".earthpress");
            if fs::create_dir_all(&dir).is_err() {
                return None;
            }
            Some(dir.join("earthpress.log"))
        })
        .as_ref()
}

fn write_entry(level: &str, message: &str, data: Option<serde_json::Value>) {
    let Some(path) = log_file() else {
        return;
    };

    let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ");
    let suffix = match data {
        Some(value) => format!(" {value}"),
        None => String::new(),
    };

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "[{timestamp}] [{level}] {message}{suffix}");
    }
}

pub fn info(message: impl AsRef<str>, data: Option<serde_json::Value>) {
    let message = message.as_ref();
    // Console output is a debugging aid only: a release build is a GUI
    // subsystem binary with no attached console.
    #[cfg(debug_assertions)]
    println!("{message}");
    write_entry("INFO", message, data);
}

pub fn warn(message: impl AsRef<str>, data: Option<serde_json::Value>) {
    let message = message.as_ref();
    #[cfg(debug_assertions)]
    eprintln!("{message}");
    write_entry("WARN", message, data);
}

pub fn error(message: impl AsRef<str>, data: Option<serde_json::Value>) {
    let message = message.as_ref();
    #[cfg(debug_assertions)]
    eprintln!("{message}");
    write_entry("ERROR", message, data);
}
