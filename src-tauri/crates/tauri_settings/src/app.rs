//! Tauri side of the crate, the managed state and the commands the frontend calls.
//!
//! The commands register themselves through link-time statics. `init` lives in the same module
//! so those statics are linked in whenever the app calls it.

use std::{collections::HashMap, error::Error, fs};

use serde::{Serialize, Serializer};
use serde_json::Value;
use tauri::{AppHandle, Manager, State};

use crate::{FILE, Settings};

/// Opens the store in the app data directory and registers it as managed state. Must run before
/// anything that reads settings. `known` is the full list of keys the app has.
pub fn init(app: &AppHandle, known: &'static [&'static str]) -> Result<(), Box<dyn Error>> {
  if app.try_state::<Settings>().is_some() {
    return Ok(());
  }
  let dir = app.path().app_data_dir()?;
  fs::create_dir_all(&dir)?;
  app.manage(Settings::open(dir.join(FILE), known));
  Ok(())
}

/// What every `settings` command rejects with. The frontend only sees the message.
#[derive(Debug)]
struct SettingsError(String);

impl Serialize for SettingsError {
  fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&self.0)
  }
}

/// Everything saved so far. Settings that were never changed are absent, the frontend has the
/// defaults.
#[tauri_groups::command(settings)]
fn get_settings(settings: State<'_, Settings>) -> HashMap<String, Value> {
  settings.all().into_iter().collect()
}

#[tauri_groups::command(settings)]
fn set_setting(
  settings: State<'_, Settings>,
  key: String,
  value: Value,
) -> Result<(), SettingsError> {
  settings
    .set(&key, value)
    .map_err(|e| SettingsError(format!("couldn't save {key:?}: {e}")))
}
