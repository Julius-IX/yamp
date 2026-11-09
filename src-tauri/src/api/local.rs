//! The local library: audio on this device that is not streamed. It is made of the downloads and
//! of the files in folders that the user added, and the songs of both are listed as one, see
//! `yamp_db::Locals`. `downloads` handles saving and removing a download, these commands deal with
//! the folders and with what the two have in common.
use std::path::PathBuf;

use super::{
  err, fail,
  wire::{LocalFolder, LocalFolderEntry, LocalUsage, tracks_from_db},
};
use crate::library;
use tauri::{AppHandle, Emitter, Manager, State};
use yamp_db::{Db, MediaStore};
use yt_api::Track;

/// The added folders, oldest first.
#[tauri_groups::command(local)]
pub async fn dirs(db: State<'_, Db>) -> Result<Vec<String>, String> {
  db.locals().dirs().await.map_err(fail)
}

/// Adds a folder and scans it in the background. Returns whether it was new. A folder that lies
/// inside another added folder, or around one, is refused, and so is one that holds the downloads.
#[tauri_groups::command(local)]
pub async fn add_dir(
  app: AppHandle,
  db: State<'_, Db>,
  media: State<'_, MediaStore>,
  path: String,
) -> Result<bool, String> {
  let dir = PathBuf::from(&path);
  if !dir.is_dir() {
    return Err(format!("{path} is not a folder"));
  }
  // the downloads are listed from their records, so the same files found by a scan would show up twice
  if dir.starts_with(media.root()) || media.root().starts_with(&dir) {
    return Err("the folder of the downloads cannot be added".into());
  }
  let added = db.locals().add_dir(&dir).await.map_err(fail)?;
  if added {
    library::allow_dir(&app, &dir);
    library::watch_dir(&app, &dir, true);
    library::scan_dirs(&app, vec![path]);
  }
  Ok(added)
}

/// Forgets a folder and its songs. The files stay where they are. Returns whether it was added.
#[tauri_groups::command(local)]
pub async fn remove_dir(app: AppHandle, db: State<'_, Db>, path: String) -> Result<bool, String> {
  library::watch_dir(&app, std::path::Path::new(&path), false);
  let removed = db.locals().remove_dir(&path).await.map_err(fail)?;
  let _ = app.emit(library::LOCAL_EVENT, ());
  Ok(removed)
}

/// Looks for new and removed files in every added folder. It returns at once, the
/// `library:local` event tells when a scan found something.
#[tauri_groups::command(local)]
pub async fn rescan(app: AppHandle) -> Result<(), String> {
  library::scan_all(&app);
  Ok(())
}

/// Where the audio of the track is on this device, ready for the asset protocol. `None` means the
/// track is only available as a stream, which includes a file that has vanished.
#[tauri_groups::command(local)]
pub async fn path(
  db: State<'_, Db>,
  media: State<'_, MediaStore>,
  track_id: String,
) -> Result<Option<String>, String> {
  let found = library::audio_path(&db, &media, &track_id).await.map_err(fail)?;
  Ok(found.map(|p| p.to_string_lossy().into_owned()))
}

/// A URL that plays the file of the track, on desktop. The webview of some desktops cannot play
/// from the asset protocol, so the file is served over loopback HTTP instead. `None` on a phone,
/// and for a track that is not on the device, `path` and the asset protocol are for the former.
#[tauri_groups::command(local)]
pub async fn url(
  serve: State<'_, library::Serve>,
  db: State<'_, Db>,
  media: State<'_, MediaStore>,
  track_id: String,
) -> Result<Option<String>, String> {
  let Some(server) = &serve.0 else { return Ok(None) };
  let found = library::audio_path(&db, &media, &track_id).await.map_err(fail)?;
  Ok(found.map(|_| server.url(&track_id)))
}

/// Where the cached covers are. A thumbnail of the form `cover:<name>` is the file `<name>.jpg`
/// in this folder.
#[tauri_groups::command(local)]
pub async fn cover_dir(app: AppHandle) -> Result<String, String> {
  Ok(library::cover_dir(&app).to_string_lossy().into_owned())
}

/// Asks for the permission to read audio files where the system wants one, and returns whether it
/// is granted. Elsewhere it returns true at once.
#[tauri_groups::command(local)]
pub async fn request_access(app: AppHandle) -> Result<bool, String> {
  // the call waits for the person to answer a prompt
  tauri::async_runtime::spawn_blocking(move || app.state::<yamp_storage::Storage>().request_audio_access())
    .await
    .map_err(err)
}

/// The folders inside `path`. Without a path it starts at the shared storage of the device. It is
/// for choosing a folder on a phone, which has no folder picker that gives back a path.
#[tauri_groups::command(local)]
pub async fn browse(path: Option<String>) -> Result<LocalFolder, String> {
  let start = path
    .map(PathBuf::from)
    .unwrap_or_else(|| PathBuf::from(if std::path::Path::new(SHARED_STORAGE).is_dir() { SHARED_STORAGE } else { "/" }));
  let listed = {
    let start = start.clone();
    tauri::async_runtime::spawn_blocking(move || yamp_db::list_subdirs(&start)).await.map_err(err)?
  };
  let dirs = listed.map_err(err)?;
  Ok(LocalFolder {
    path: start.to_string_lossy().into_owned(),
    parent: start.parent().map(|p| p.to_string_lossy().into_owned()),
    dirs: dirs.into_iter().map(|d| LocalFolderEntry { name: d.name, path: d.path }).collect(),
  })
}

/// Where the music of a phone is, whatever the user's account number is.
const SHARED_STORAGE: &str = "/storage/emulated/0";

/// The local songs, newest first.
#[tauri_groups::command(local)]
pub async fn tracks(db: State<'_, Db>, limit: u32, offset: u32) -> Result<Vec<Track>, String> {
  Ok(tracks_from_db(db.locals().tracks(limit, offset).await.map_err(fail)?))
}

#[tauri_groups::command(local)]
pub async fn count(db: State<'_, Db>) -> Result<u64, String> {
  db.locals().count().await.map_err(fail)
}

#[tauri_groups::command(local)]
pub async fn usage(db: State<'_, Db>) -> Result<LocalUsage, String> {
  Ok(db.locals().usage().await.map_err(fail)?.into())
}
