//! Central log hub shared by the Rust backend and the web frontend.
//!
//! Backend code logs through the `log` facade. The frontend logs through the `logging::push`
//! command. Both end up in one ring buffer, are forwarded to the platform log (see `sink`) and are
//! emitted as events, so any window can show every entry and still split on `LogSource`.
//!
//! The buffer lives in the backend on purpose. A window that opens late or reloads can fetch the
//! history with `logging::list` instead of losing everything logged before it existed.

mod commands;
mod sink;

use std::{
  cell::Cell,
  collections::VecDeque,
  sync::{
    Mutex, OnceLock,
    atomic::{AtomicU64, Ordering},
  },
  time::{SystemTime, UNIX_EPOCH},
};

use log::{Level, LevelFilter, Log, Metadata, Record};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use ts_types::TsType;

/// Event carrying a single `LogEntry`.
pub const ENTRY_EVENT: &str = "log:entry";
/// Event sent after the buffer was emptied.
pub const CLEAR_EVENT: &str = "log:clear";

/// Entries kept for late subscribers. The oldest are dropped first.
const CAPACITY: usize = 5000;

#[derive(Clone, Copy, Serialize, Deserialize, TsType)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
  Error,
  Warn,
  Info,
  Debug,
  Trace,
}

#[derive(Clone, Copy, Serialize, TsType)]
#[serde(rename_all = "lowercase")]
pub enum LogSource {
  Backend,
  Frontend,
}

#[derive(Clone, Serialize, TsType)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
  /// Increases per entry, so a consumer can drop duplicates when merging history and events.
  id: u64,
  /// Unix time in milliseconds.
  ts: u64,
  level: LogLevel,
  source: LogSource,
  target: String,
  message: String,
}

impl From<Level> for LogLevel {
  fn from(l: Level) -> Self {
    match l {
      Level::Error => Self::Error,
      Level::Warn => Self::Warn,
      Level::Info => Self::Info,
      Level::Debug => Self::Debug,
      Level::Trace => Self::Trace,
    }
  }
}

impl From<LogLevel> for Level {
  fn from(l: LogLevel) -> Self {
    match l {
      LogLevel::Error => Self::Error,
      LogLevel::Warn => Self::Warn,
      LogLevel::Info => Self::Info,
      LogLevel::Debug => Self::Debug,
      LogLevel::Trace => Self::Trace,
    }
  }
}

static BUFFER: Mutex<VecDeque<LogEntry>> = Mutex::new(VecDeque::new());
static NEXT_ID: AtomicU64 = AtomicU64::new(1);
/// Unset until the app is built. Entries logged before that are buffered but not emitted.
static HANDLE: OnceLock<AppHandle> = OnceLock::new();

struct Logger;

impl Log for Logger {
  fn enabled(&self, metadata: &Metadata) -> bool {
    metadata.level() <= log::max_level()
  }

  fn log(&self, record: &Record) {
    if self.enabled(record.metadata()) {
      push(
        LogSource::Backend,
        record.level().into(),
        record.target(),
        record.args().to_string(),
      );
    }
  }

  fn flush(&self) {}
}

/// Installs the logger. Must run before anything that logs, so before the Tauri builder.
pub fn init() {
  // a second logger can only be set in tests, where the first one is kept
  if log::set_logger(&Logger).is_ok() {
    log::set_max_level(if cfg!(debug_assertions) {
      LevelFilter::Debug
    } else {
      LevelFilter::Info
    });
  }
}

/// Starts emitting entries to the frontend. Called once the app exists.
pub fn attach(handle: AppHandle) {
  let _ = HANDLE.set(handle);
}

fn lock() -> std::sync::MutexGuard<'static, VecDeque<LogEntry>> {
  // a panic while logging must not silence every later entry
  BUFFER.lock().unwrap_or_else(|e| e.into_inner())
}

fn push(source: LogSource, level: LogLevel, target: &str, message: String) {
  thread_local! {
    static BUSY: Cell<bool> = const { Cell::new(false) };
  }
  // Emitting runs Tauri code that can log itself. Without this guard that would recurse forever.
  if BUSY.replace(true) {
    return;
  }

  let entry = LogEntry {
    id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
    ts: SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .map_or(0, |d| d.as_millis() as u64),
    level,
    source,
    target: target.to_owned(),
    message,
  };

  sink::write(&entry);
  {
    let mut buf = lock();
    if buf.len() == CAPACITY {
      buf.pop_front();
    }
    buf.push_back(entry.clone());
  }
  if let Some(handle) = HANDLE.get() {
    let _ = handle.emit(ENTRY_EVENT, &entry);
  }

  BUSY.set(false);
}

fn snapshot() -> Vec<LogEntry> {
  lock().iter().cloned().collect()
}

fn clear() {
  lock().clear();
  if let Some(handle) = HANDLE.get() {
    let _ = handle.emit(CLEAR_EVENT, ());
  }
}
