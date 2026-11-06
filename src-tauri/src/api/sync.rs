//! Linking the library to other devices through the cloud account of the user. `sign_in` runs the
//! login of the service in the browser, and the link is restored from the saved tokens at the
//! next start. A plain folder is still supported by `connect_folder`. A sync only runs when the
//! frontend asks for it, and it should refresh its lists when `pulled` is not 0.
use std::{path::PathBuf, time::Duration};

use super::{
  err, fail,
  wire::{SyncDevice, SyncProvider, SyncReport, SyncStatus},
};
use crate::library::{self, Connected, SYNC_KEY, SyncConfig, SyncState};
use cloud_auth::TokenStore;
use cloud_sync::Provider;
use serde_json::Value;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use tauri_settings::Settings;
use yamp_db::{Db, Link};

const NOT_CONNECTED: &str = "the library is not linked to anything";

/// Long enough to find the password and a second factor, short enough that a forgotten sign in
/// does not keep the link busy all day.
const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(5 * 60);

/// The persisted name of a provider, the same one the settings file holds.
fn provider_id(provider: Provider) -> String {
  serde_json::to_value(provider).ok().and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_default()
}

fn parse_provider(id: &str) -> Result<Provider, String> {
  serde_json::from_value(Value::String(id.to_owned())).map_err(|_| format!("unknown service {id}"))
}

fn link_of(sync: &SyncState) -> Result<Link, String> {
  sync.get().map(|c| c.link).ok_or_else(|| NOT_CONNECTED.into())
}

fn status_of(connected: Option<Connected>, tokens: &TokenStore) -> SyncStatus {
  let Some(c) = connected else {
    return SyncStatus::default();
  };
  let folder = c.config.folder.to_string_lossy().into_owned();
  SyncStatus {
    connected: true,
    provider: c.config.provider.map(provider_id),
    account: c.config.provider.and_then(|p| tokens.load(p)).and_then(|s| s.account),
    folder: Some(folder).filter(|f| !f.is_empty()),
    device_name: Some(c.config.device_name),
    device_id: Some(c.link.device_id().to_owned()),
  }
}

#[tauri_groups::command(sync)]
pub fn status(sync: State<'_, SyncState>, tokens: State<'_, TokenStore>) -> SyncStatus {
  status_of(sync.get(), &tokens)
}

/// The services this build can link to on this platform. One whose OAuth client was not set when
/// the app was built is left out, so the list is empty for a build without any.
#[tauri_groups::command(sync)]
pub fn providers() -> Vec<SyncProvider> {
  Provider::available()
    .iter()
    .filter(|&&p| match cloud_auth::check(p) {
      Ok(()) => true,
      // the settings only show what is there, so the reason is left in the log
      Err(e) => {
        log::info!("{e}");
        false
      }
    })
    .map(|&p| SyncProvider { id: provider_id(p), name: cloud_auth::display_name(p).to_owned() })
    .collect()
}

/// Signs in to a cloud service and links the library to the account. The login opens in the
/// browser, and the call returns once it is done, refused, cancelled or timed out. Nothing is
/// sent or received yet, that is what `sync.now` does.
#[tauri_groups::command(sync)]
pub async fn sign_in(
  app: AppHandle,
  db: State<'_, Db>,
  settings: State<'_, Settings>,
  sync: State<'_, SyncState>,
  tokens: State<'_, TokenStore>,
  provider: String,
  device_name: String,
) -> Result<SyncStatus, String> {
  let provider = parse_provider(&provider)?;
  let _busy = sync.begin()?;
  let cancel = sync.new_sign_in();

  // the identifier doubles as the scheme of the link a mobile sign in is sent back by
  let pending = cloud_auth::begin(provider, &app.config().identifier).map_err(err)?;
  app.opener().open_url(pending.url.clone(), None::<&str>).map_err(err)?;
  // the wait blocks on a socket, so it runs on a thread that is allowed to
  let signed = tauri::async_runtime::spawn_blocking(move || pending.wait(SIGN_IN_TIMEOUT, &cancel))
    .await
    .map_err(err)?
    .map_err(err)?;

  let session = cloud_auth::Session::from_sign_in(provider, tokens.inner().clone(), signed).map_err(err)?;
  let store = library::cloud_store(provider, session)?;
  let link = library::link(&db, &device_name, store).await?;

  // a library that was linked to another service leaves its tokens behind otherwise
  if let Some(old) = sync.get().and_then(|c| c.config.provider).filter(|&p| p != provider) {
    tokens.remove(old).map_err(err)?;
  }
  let config = SyncConfig { folder: PathBuf::new(), provider: Some(provider), device_name };
  settings.set(SYNC_KEY, serde_json::to_value(&config).map_err(err)?).map_err(err)?;
  sync.set(Some(Connected { link, config }));
  Ok(status_of(sync.get(), &tokens))
}

/// Abandons a `sign_in` that is waiting for the browser.
#[tauri_groups::command(sync)]
pub fn cancel_sign_in(sync: State<'_, SyncState>) {
  sync.cancel_sign_in();
}

/// Links the library to the folder and remembers it for the next start. Nothing is sent or
/// received yet, that is what `sync.now` does.
#[tauri_groups::command(sync)]
pub async fn connect_folder(
  db: State<'_, Db>,
  settings: State<'_, Settings>,
  sync: State<'_, SyncState>,
  tokens: State<'_, TokenStore>,
  folder: String,
  device_name: String,
) -> Result<SyncStatus, String> {
  let _busy = sync.begin()?;
  let config = SyncConfig { folder: folder.into(), provider: None, device_name };
  let link = library::connect(&db, &config, &tokens).await?;
  settings.set(SYNC_KEY, serde_json::to_value(&config).map_err(err)?).map_err(err)?;
  sync.set(Some(Connected { link, config }));
  Ok(status_of(sync.get(), &tokens))
}

/// Stops syncing and signs out. What is in the cloud stays there, and so does the library of this
/// device.
#[tauri_groups::command(sync)]
pub fn disconnect(
  settings: State<'_, Settings>,
  sync: State<'_, SyncState>,
  tokens: State<'_, TokenStore>,
) -> Result<(), String> {
  let _busy = sync.begin()?;
  if let Some(provider) = sync.get().and_then(|c| c.config.provider) {
    tokens.remove(provider).map_err(err)?;
  }
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
