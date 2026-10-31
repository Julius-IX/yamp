//! The settings file and its in-memory copy.
//!
//! Every save rewrites the whole file through a temp file and a rename, so a crash mid-save
//! leaves either the old or the new file and never a half written one.

use std::{
  fs, io,
  io::Write,
  path::{Path, PathBuf},
  sync::{Arc, Mutex, MutexGuard},
};

use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

/// Name of the file inside the app data directory.
pub const FILE: &str = "settings.json";

/// Shared handle to the store. Clones point at the same values.
#[derive(Clone)]
pub struct Settings(Arc<Inner>);

struct Inner {
  path: PathBuf,
  known: &'static [&'static str],
  values: Mutex<Map<String, Value>>,
}

impl Settings {
  /// Loads the file at `path`. A missing or unreadable file means no settings yet. Keys that
  /// are not in `known` are dropped, and the file is rewritten without them right away.
  pub fn open(path: PathBuf, known: &'static [&'static str]) -> Self {
    let mut values = read(&path);
    let before = values.len();
    values.retain(|k, _| known.contains(&k.as_str()));
    if values.len() != before {
      // retried on the next load if this fails
      let _ = save(&path, &values);
    }
    Settings(Arc::new(Inner {
      path,
      known,
      values: Mutex::new(values),
    }))
  }

  /// Everything saved so far. Settings that were never changed are absent.
  pub fn all(&self) -> Map<String, Value> {
    self.lock().clone()
  }

  /// The value saved under `key`, or `None` if there is none or it is not a `T`.
  pub fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
    serde_json::from_value(self.lock().get(key)?.clone()).ok()
  }

  /// Saves `value` under `key`. Nothing changes if the key is unknown or the file can't be
  /// written.
  pub fn set(&self, key: &str, value: Value) -> io::Result<()> {
    if !self.0.known.contains(&key) {
      return Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "not a setting this app has",
      ));
    }
    // The lock is held while writing, so saves reach the disk in the order they were made.
    let mut values = self.lock();
    let mut next = values.clone();
    next.insert(key.to_string(), value);
    save(&self.0.path, &next)?;
    *values = next;
    Ok(())
  }

  /// Replaces every setting at once, for example when a backup is imported. Unknown keys are
  /// dropped like on load. Nothing changes if the file can't be written.
  pub fn replace(&self, mut next: Map<String, Value>) -> io::Result<()> {
    next.retain(|k, _| self.0.known.contains(&k.as_str()));
    let mut values = self.lock();
    save(&self.0.path, &next)?;
    *values = next;
    Ok(())
  }

  fn lock(&self) -> MutexGuard<'_, Map<String, Value>> {
    // a panic while holding the lock must not make every later read and write panic
    self.0.values.lock().unwrap_or_else(|e| e.into_inner())
  }
}

fn read(path: &Path) -> Map<String, Value> {
  let text = match fs::read_to_string(path) {
    Ok(text) => text,
    // first run
    Err(e) if e.kind() == io::ErrorKind::NotFound => return Map::new(),
    Err(e) => {
      log::warn!("couldn't read {}: {e}, using defaults", path.display());
      return Map::new();
    }
  };
  match serde_json::from_str::<Value>(&text) {
    Ok(Value::Object(map)) => map,
    other => {
      let why = match other {
        Err(e) => e.to_string(),
        _ => "not a JSON object".to_string(),
      };
      // kept aside instead of deleted, so a hand edit gone wrong can still be recovered
      let aside = path.with_extension("json.corrupt");
      log::warn!(
        "{} is unusable ({why}), moved to {}",
        path.display(),
        aside.display()
      );
      let _ = fs::rename(path, aside);
      Map::new()
    }
  }
}

/// Writes to `settings.json.tmp`, flushes it to disk, then renames it over the real file.
fn save(path: &Path, values: &Map<String, Value>) -> io::Result<()> {
  if let Some(dir) = path.parent() {
    fs::create_dir_all(dir)?;
  }
  let tmp = path.with_extension("json.tmp");
  let written = fs::File::create(&tmp).and_then(|mut f| {
    f.write_all(serde_json::to_string_pretty(values)?.as_bytes())?;
    f.sync_all()
  });
  written
    .and_then(|_| fs::rename(&tmp, path))
    .inspect_err(|_| {
      let _ = fs::remove_file(&tmp);
    })
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  const KNOWN: &[&str] = &["theme", "mode"];

  fn temp_file() -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("tauri-settings-{}-{n}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir.join(FILE)
  }

  fn open(path: PathBuf) -> Settings {
    Settings::open(path, KNOWN)
  }

  #[test]
  fn nothing_is_set_on_a_fresh_install() {
    assert!(open(temp_file()).all().is_empty());
  }

  #[test]
  fn a_setting_survives_a_restart() {
    let path = temp_file();
    open(path.clone()).set("theme", json!("ocean")).unwrap();
    assert_eq!(open(path).get::<String>("theme").as_deref(), Some("ocean"));
  }

  #[test]
  fn a_value_of_the_wrong_type_reads_as_none() {
    let s = open(temp_file());
    s.set("theme", json!(3)).unwrap();
    assert_eq!(s.get::<String>("theme"), None);
    assert_eq!(s.get::<u8>("theme"), Some(3));
  }

  #[test]
  fn a_corrupt_file_is_kept_aside_and_saving_still_works() {
    let path = temp_file();
    fs::write(&path, "{{nope").unwrap();

    let s = open(path.clone());
    assert!(s.all().is_empty());
    assert_eq!(
      fs::read_to_string(path.with_extension("json.corrupt")).unwrap(),
      "{{nope"
    );

    s.set("theme", json!("ocean")).unwrap();
    assert_eq!(open(path).get::<String>("theme").as_deref(), Some("ocean"));
  }

  #[test]
  fn a_file_that_is_not_an_object_counts_as_corrupt() {
    let path = temp_file();
    fs::write(&path, "[1, 2]").unwrap();
    assert!(open(path.clone()).all().is_empty());
    assert!(path.with_extension("json.corrupt").exists());
  }

  #[test]
  fn a_leftover_temp_file_from_a_crash_is_harmless() {
    let path = temp_file();
    open(path.clone()).set("theme", json!("ocean")).unwrap();
    // crash mid-save
    fs::write(path.with_extension("json.tmp"), "{half a wri").unwrap();

    let s = open(path.clone());
    assert_eq!(s.get::<String>("theme").as_deref(), Some("ocean"));
    s.set("mode", json!("dark")).unwrap();
    assert_eq!(open(path).get::<String>("mode").as_deref(), Some("dark"));
  }

  #[test]
  fn keys_the_app_no_longer_has_are_dropped_from_the_file() {
    let path = temp_file();
    fs::write(
      &path,
      r#"{"theme": "ocean", "oldSetting": 1, "typo": true}"#,
    )
    .unwrap();

    let s = open(path.clone());
    assert_eq!(s.all().len(), 1);
    // gone from the file itself right away, not only from memory
    let on_disk: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(on_disk, json!({ "theme": "ocean" }));
  }

  #[test]
  fn replace_swaps_everything_and_drops_unknown_keys() {
    let path = temp_file();
    let s = open(path.clone());
    s.set("theme", json!("ocean")).unwrap();
    s.set("mode", json!("dark")).unwrap();

    let mut next = Map::new();
    next.insert("theme".into(), json!("forest"));
    next.insert("bogus".into(), json!(1));
    s.replace(next).unwrap();

    assert_eq!(s.get::<String>("theme").as_deref(), Some("forest"));
    // not in the replacement, so gone
    assert_eq!(s.get::<String>("mode"), None);
    let on_disk: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(on_disk, json!({ "theme": "forest" }));
  }

  #[test]
  fn an_unknown_key_cannot_be_saved() {
    let s = open(temp_file());
    assert!(s.set("whatever", json!(1)).is_err());
    assert!(s.all().is_empty());
  }

  #[test]
  fn clones_share_the_same_values() {
    let a = open(temp_file());
    let b = a.clone();
    a.set("theme", json!("ocean")).unwrap();
    assert_eq!(b.get::<String>("theme").as_deref(), Some("ocean"));
  }

  #[test]
  fn a_failed_save_changes_nothing() {
    // a directory where the file should be makes the rename fail
    let path = temp_file();
    fs::create_dir_all(&path).unwrap();
    let s = open(path);
    assert!(s.set("theme", json!("ocean")).is_err());
    assert_eq!(s.get::<String>("theme"), None);
  }
}
