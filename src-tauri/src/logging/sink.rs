//! Platform log output. Entries are always kept in the in-app buffer, the sink only adds the
//! native place a platform expects logs to be, so `adb logcat` and Console.app show them too.
//! Every platform also gets a log file, since the buffer is gone once the app closes.

use super::LogEntry;
use log::Level;
use std::{
  fs::{self, File},
  io::Write,
  path::Path,
  sync::Mutex,
};
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
  write_file(entry);
}

#[cfg(target_os = "ios")]
pub fn write(entry: &LogEntry) {
  use std::sync::OnceLock;
  static SINK: OnceLock<oslog::OsLogger> = OnceLock::new();
  let sink = SINK.get_or_init(|| oslog::OsLogger::new("com.salad.yamp"));
  with_record(entry, |r| sink.log(r));
  write_file(entry);
}

/// Empty until `open_file` ran. Entries logged before that only reach the other outputs.
static FILE: Mutex<Option<File>> = Mutex::new(None);

const FILE_NAME: &str = "yamp.log";
/// The log of the run before, so a crash is still readable after the app was started again.
const PREVIOUS_NAME: &str = "yamp.previous.log";

/// Starts the log file in `dir` and writes `backlog` into it, the entries from before the
/// directory was known. The file of the last run is kept once as the previous one.
pub fn open_file(dir: &Path, backlog: &[LogEntry]) {
  let opened = fs::create_dir_all(dir).and_then(|()| {
    let path = dir.join(FILE_NAME);
    if path.exists() {
      fs::rename(&path, dir.join(PREVIOUS_NAME))?;
    }
    File::create(path)
  });
  match opened {
    Ok(mut file) => {
      for entry in backlog {
        let _ = file.write_all(line(entry).as_bytes());
      }
      *FILE.lock().unwrap_or_else(|e| e.into_inner()) = Some(file);
    }
    // there is no file to receive this entry, so it only reaches the other outputs
    Err(e) => log::warn!("couldn't open the log file in {}: {e}", dir.display()),
  }
}

fn write_file(entry: &LogEntry) {
  if let Some(file) = FILE.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
    // a full disk must not take the app down, the other outputs still have the entry
    let _ = file.write_all(line(entry).as_bytes());
  }
}

fn line(entry: &LogEntry) -> String {
  let level = Level::from(entry.level);
  format!("{} [{level:<5}] {}: {}\n", stamp(entry.ts), entry.target, entry.message)
}

/// `2026-11-11T10:51:05.707Z`. Done by hand to keep a date crate out of the dependencies.
fn stamp(ms: u64) -> String {
  let (days, rest) = (ms / 86_400_000, ms % 86_400_000);
  // civil date from a day count, the algorithm of Howard Hinnant
  let z = days as i64 + 719_468;
  let era = z.div_euclid(146_097);
  let doe = z.rem_euclid(146_097);
  let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
  let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
  let mp = (5 * doy + 2) / 153;
  let day = doy - (153 * mp + 2) / 5 + 1;
  let month = if mp < 10 { mp + 3 } else { mp - 9 };
  let year = yoe + era * 400 + i64::from(month <= 2);
  let (h, m, s, milli) = (rest / 3_600_000, rest / 60_000 % 60, rest / 1000 % 60, rest % 1000);
  format!("{year:04}-{month:02}-{day:02}T{h:02}:{m:02}:{s:02}.{milli:03}Z")
}

/// Linux, Windows and macOS. A release build on Windows has no console, so stderr goes nowhere
/// there and only the in-app view and the log file show the entries.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub fn write(entry: &LogEntry) {
  let level = Level::from(entry.level);
  eprintln!("[{level:<5}] {}: {}", entry.target, entry.message);
  write_file(entry);
}

#[cfg(test)]
mod tests {
  use super::stamp;

  #[test]
  fn stamp_is_utc_iso() {
    assert_eq!(stamp(0), "1970-01-01T00:00:00.000Z");
    assert_eq!(stamp(1_793_094_665_707), "2026-10-27T09:51:05.707Z");
  }

  #[test]
  fn stamp_handles_a_leap_day() {
    assert_eq!(stamp(951_782_400_000), "2000-02-29T00:00:00.000Z");
  }
}
