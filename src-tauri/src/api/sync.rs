//! Linking the library to other devices. The folder is chosen by the user and kept equal between
//! devices by something outside the app, see `library::connect`. A sync only runs when the
//! frontend asks for it, and it should refresh its lists when `pulled` is not 0.
use super::{
  err, fail,
  wire::{SyncDevice, SyncReport, SyncStatus},
};
use crate::library::{self, Connected, SYNC_KEY, SyncConfig, SyncState};
use serde_json::Value;
use tauri::State;
use tauri_settings::Settings;
use yamp_db::{Db, Link};

const NOT_CONNECTED: &str = "no folder is connected";

fn link_of(sync: &SyncState) -> Result<Link, String> {
  sync.get().map(|c| c.link).ok_or_else(|| NOT_CONNECTED.into())
}

fn status_of(connected: Option<Connected>) -> SyncStatus {
  let Some(c) = connected else {
    return SyncStatus::default();
  };
  SyncStatus {
    connected: true,
    folder: Some(c.config.folder.to_string_lossy().into_owned()),
    device_name: Some(c.config.device_name),
    device_id: Some(c.link.device_id().to_owned()),
  }
}

#[tauri_groups::command(sync)]
pub fn status(sync: State<'_, SyncState>) -> SyncStatus {
  status_of(sync.get())
}

/// Links the library to the folder and remembers it for the next start. Nothing is sent or
/// received yet, that is what `sync.now` does.
#[tauri_groups::command(sync)]
pub async fn connect_folder(
  db: State<'_, Db>,
  settings: State<'_, Settings>,
  sync: State<'_, SyncState>,
  folder: String,
  device_name: String,
) -> Result<SyncStatus, String> {
  let _busy = sync.begin()?;
  let config = SyncConfig { folder: folder.into(), device_name };
  let link = library::connect(&db, &config).await?;
  settings.set(SYNC_KEY, serde_json::to_value(&config).map_err(err)?).map_err(err)?;
  sync.set(Some(Connected { link, config }));
  Ok(status_of(sync.get()))
}

/// Stops syncing. What is in the folder stays there, and so does the library of this device.
#[tauri_groups::command(sync)]
pub fn disconnect(settings: State<'_, Settings>, sync: State<'_, SyncState>) -> Result<(), String> {
  let _busy = sync.begin()?;
  settings.set(SYNC_KEY, Value::Null).map_err(err)?;
  sync.set(None);
  Ok(())
}

/// Receives, then sends. A device that missed deleted log entries starts from a snapshot first.
#[tauri_groups::command(sync)]
pub async fn now(sync: State<'_, SyncState>) -> Result<SyncReport, String> {
  let link = link_of(&sync)?;
  let _busy = sync.begin()?;
  Ok(link.sync().await.map_err(fail)?.into())
}

/// Writes a snapshot and removes log entries that every active device has read. Meant to run now
/// and then, for example weekly, and not on every sync.
#[tauri_groups::command(sync)]
pub async fn compact(sync: State<'_, SyncState>) -> Result<(), String> {
  let link = link_of(&sync)?;
  let _busy = sync.begin()?;
  link.compact().await.map_err(fail)
}

/// Every device that shares the library, this one included.
#[tauri_groups::command(sync)]
pub async fn devices(sync: State<'_, SyncState>) -> Result<Vec<SyncDevice>, String> {
  let devices = link_of(&sync)?.devices().await.map_err(fail)?;
  Ok(devices.into_iter().map(Into::into).collect())
}

/// Removes another device from the folder, for example one that was lost. A device cannot forget
/// itself.
#[tauri_groups::command(sync)]
pub async fn forget_device(sync: State<'_, SyncState>, id: String) -> Result<(), String> {
  let link = link_of(&sync)?;
  let _busy = sync.begin()?;
  link.forget_device(&id).await.map_err(fail)
}
