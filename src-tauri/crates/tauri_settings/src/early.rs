//! Reading a single key before the Tauri app exists.
//!
//! Some choices have to be made before the app is built, such as the GTK backend on Linux. The
//! managed [`Settings`](crate::Settings) is not available that early.

use std::{fs, path::Path};

use serde::de::DeserializeOwned;
use serde_json::Value;

/// Reads `key` from the settings file at `path` and changes nothing. No keys are pruned, a
/// corrupt file is not moved aside and nothing is created. A missing file, an unreadable one
/// and a value of the wrong type all give `None`.
pub fn peek<T: DeserializeOwned>(path: &Path, key: &str) -> Option<T> {
  let Value::Object(map) = serde_json::from_str::<Value>(&fs::read_to_string(path).ok()?).ok()?
  else {
    return None;
  };
  serde_json::from_value(map.get(key)?.clone()).ok()
}

/// [`peek`] for the file of the app with bundle `identifier`. The file is found the way Tauri's
/// `app_data_dir()` does on Linux, which is `$XDG_DATA_HOME` (if it is an absolute path) or
/// `~/.local/share`, then the identifier. Only Linux needs this, other platforms have a fixed
/// location the app can build itself.
#[cfg(target_os = "linux")]
pub fn peek_early<T: DeserializeOwned>(identifier: &str, key: &str) -> Option<T> {
  use std::{env::var_os, path::PathBuf};

  let data_home = var_os("XDG_DATA_HOME")
    .map(PathBuf::from)
    .filter(|p| p.is_absolute())
    .or_else(|| var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))?;
  peek(&data_home.join(identifier).join(crate::FILE), key)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn temp_file(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
      "tauri-settings-early-{}-{name}",
      std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir.join(crate::FILE)
  }

  #[test]
  fn peek_reads_one_key_and_changes_nothing() {
    let path = temp_file("reads");
    fs::write(&path, r#"{"flag": false, "oldSetting": 1}"#).unwrap();

    assert_eq!(peek::<bool>(&path, "flag"), Some(false));
    // never saved
    assert_eq!(peek::<bool>(&path, "other"), None);
    // wrong type
    assert_eq!(peek::<String>(&path, "flag"), None);
    // untouched, opening the store would have pruned the stale key
    assert!(fs::read_to_string(&path).unwrap().contains("oldSetting"));
  }

  #[test]
  fn peek_on_a_missing_or_corrupt_file_is_none() {
    let path = temp_file("missing");
    assert_eq!(peek::<bool>(&path, "flag"), None);

    fs::write(&path, "{{nope").unwrap();
    assert_eq!(peek::<bool>(&path, "flag"), None);
    // left for the store to deal with
    assert!(!path.with_extension("json.corrupt").exists());
  }
}
