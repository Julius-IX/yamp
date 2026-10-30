use super::{LogEntry, LogLevel, LogSource};

/// History for a view that opens after entries were already logged.
#[tauri_groups::command(logging)]
pub fn list() -> Vec<LogEntry> {
  super::snapshot()
}

/// Entry point of the frontend logger. Levels above the backend filter are dropped here, so both
/// sides share one verbosity setting.
#[tauri_groups::command(logging)]
pub fn push(level: LogLevel, target: String, message: String) {
  if log::Level::from(level) <= log::max_level() {
    super::push(LogSource::Frontend, level, &target, message);
  }
}

#[tauri_groups::command(logging)]
pub fn clear() {
  super::clear();
}
