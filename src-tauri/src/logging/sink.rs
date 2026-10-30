//! Platform log output. Entries are always kept in the in-app buffer, the sink only adds the
//! native place a platform expects logs to be, so `adb logcat` and Console.app show them too.

use super::LogEntry;
use log::Level;
#[cfg(any(target_os = "android", target_os = "ios"))]
use log::{Log, Record};

/// The native loggers take a `Record`, which borrows its arguments and cannot be returned.
#[cfg(any(target_os = "android", target_os = "ios"))]
fn with_record(entry: &LogEntry, f: impl FnOnce(&Record)) {
  f(&Record::builder()
    .level(Level::from(entry.level))
    .target(&entry.target)
    .args(format_args!("{}", entry.message))
    .build());
}

#[cfg(target_os = "android")]
pub fn write(entry: &LogEntry) {
  use std::sync::OnceLock;
  static SINK: OnceLock<android_logger::AndroidLogger> = OnceLock::new();
  let sink = SINK.get_or_init(|| {
    android_logger::AndroidLogger::new(android_logger::Config::default().with_tag("yamp"))
  });
  with_record(entry, |r| sink.log(r));
}

#[cfg(target_os = "ios")]
pub fn write(entry: &LogEntry) {
  use std::sync::OnceLock;
  static SINK: OnceLock<oslog::OsLogger> = OnceLock::new();
  let sink = SINK.get_or_init(|| oslog::OsLogger::new("com.salad.yamp"));
  with_record(entry, |r| sink.log(r));
}

/// Linux, Windows and macOS. A release build on Windows has no console, so stderr goes nowhere
/// there and the in-app view is the only output.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub fn write(entry: &LogEntry) {
  let level = Level::from(entry.level);
  eprintln!("[{level:<5}] {}: {}", entry.target, entry.message);
}
