//! Updating the app in place. The `tauri-plugin-updater` plugin reads a manifest from a release,
//! checks the signature of the bundle and swaps it in. The plugin is only registered when this
//! install can use it, see `init`, and `available` tells the frontend which case it is in.
//!
//! A build needs the public key of the signing key in `plugins.updater.pubkey` of
//! `tauri.conf.json`, and the private key in `TAURI_SIGNING_PRIVATE_KEY` while it is bundled.
//! Linux can only update an AppImage, since any other install belongs to a package manager.
//! A phone has no plugin. Android finds new releases from the frontend, since an APK is installed
//! by the system and not by the app.
use serde::Serialize;
use tauri::AppHandle;
use ts_types::TsType;

#[cfg(desktop)]
use super::err;

/// Event carrying an `UpdateProgress` while an update downloads.
pub const PROGRESS_EVENT: &str = "updater:progress";

#[derive(Clone, Serialize, TsType)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
  pub version: String,
  /// The release notes, when the release has any.
  pub notes: Option<String>,
}

#[derive(Clone, Serialize, TsType)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProgress {
  pub downloaded: u64,
  /// Unknown when the server does not send a length.
  pub total: Option<u64>,
}

#[cfg(desktop)]
mod desktop {
  use std::sync::Mutex;

  use tauri::{AppHandle, Emitter, Manager};
  use tauri_plugin_updater::{Update, UpdaterExt};

  use super::{PROGRESS_EVENT, UpdateInfo, UpdateProgress, err};

  const NOT_AVAILABLE: &str = "updates are not available in this install";

  /// The update that `check` found, kept for `install`. Its presence in the app state is also what
  /// marks an install as able to update.
  #[derive(Default)]
  pub struct Updates(Mutex<Option<Update>>);

  /// Registers the plugin when the config has a public key and the install can be replaced.
  /// Without the plugin every check would fail, which is why it is left out instead.
  pub fn init(app: &tauri::App) -> tauri::Result<()> {
    let key = app
      .config()
      .plugins
      .0
      .get("updater")
      .and_then(|c| c.get("pubkey"))
      .and_then(|k| k.as_str())
      .unwrap_or_default();
    if key.trim().is_empty() {
      log::info!("updates are off, the build has no public key");
      return Ok(());
    }
    #[cfg(target_os = "linux")]
    if app.env().appimage.is_none() {
      log::info!("updates are off, only an AppImage can replace itself");
      return Ok(());
    }
    app.handle().plugin(tauri_plugin_updater::Builder::new().build())?;
    app.manage(Updates::default());
    Ok(())
  }

  pub fn available(app: &AppHandle) -> bool {
    app.try_state::<Updates>().is_some()
  }

  pub async fn check(app: &AppHandle) -> Result<Option<UpdateInfo>, String> {
    let updates = app.try_state::<Updates>().ok_or(NOT_AVAILABLE)?;
    let found = app.updater().map_err(err)?.check().await.map_err(err)?;
    let info = found.as_ref().map(|u| UpdateInfo { version: u.version.clone(), notes: u.body.clone() });
    if let Some(info) = &info {
      log::info!("update {} is available", info.version);
    }
    *updates.0.lock().unwrap_or_else(|e| e.into_inner()) = found;
    Ok(info)
  }

  pub async fn install(app: &AppHandle) -> Result<(), String> {
    let updates = app.try_state::<Updates>().ok_or(NOT_AVAILABLE)?;
    let update = updates.0.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let update = update.ok_or("no update was found, check for one first")?;
    let (handle, mut downloaded) = (app.clone(), 0u64);
    update
      .download_and_install(
        move |chunk, total| {
          downloaded += chunk as u64;
          let _ = handle.emit(PROGRESS_EVENT, UpdateProgress { downloaded, total });
        },
        || {},
      )
      .await
      .map_err(err)?;
    log::info!("update {} is installed", update.version);
    Ok(())
  }
}

/// Called once from the setup of the app.
pub fn init(app: &tauri::App) -> tauri::Result<()> {
  #[cfg(desktop)]
  return desktop::init(app);
  #[cfg(mobile)]
  {
    let _ = app;
    Ok(())
  }
}

/// Whether this install can update itself. False on a phone, the frontend handles Android on its
/// own, and false for a build without a public key.
#[tauri_groups::command(updater)]
pub fn available(app: AppHandle) -> bool {
  #[cfg(desktop)]
  return desktop::available(&app);
  #[cfg(mobile)]
  {
    let _ = app;
    false
  }
}

/// Looks for a newer release. The one that is found is kept for `install`.
#[tauri_groups::command(updater)]
pub async fn check(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
  #[cfg(desktop)]
  return desktop::check(&app).await;
  #[cfg(mobile)]
  {
    let _ = app;
    Ok(None)
  }
}

/// Downloads and installs what `check` found. The `updater:progress` event reports the download.
/// Windows closes the app once its installer has started, the other platforms need `restart`.
#[tauri_groups::command(updater)]
pub async fn install(app: AppHandle) -> Result<(), String> {
  #[cfg(desktop)]
  return desktop::install(&app).await;
  #[cfg(mobile)]
  {
    let _ = app;
    Err("updates are not available on a phone".into())
  }
}

#[tauri_groups::command(updater)]
pub fn restart(app: AppHandle) {
  app.restart()
}
