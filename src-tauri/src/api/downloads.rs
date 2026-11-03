//! Downloaded audio of the library. `queue` shows the jobs of the provider queue, these commands
//! deal with what ends up on disk.
use super::{
  fail,
  wire::{DownloadRecord, track_to_db, tracks_from_db},
};
use tauri::State;
use yamp_db::{Db, MediaStore};
use yt_api::{AudioQuality, DownloadJob, Track, YtApi};

/// Saves the metadata of the track and queues the download. A finished download is moved into the
/// library folder, so `DownloadJob.file` points at a file that is gone by then. The
/// `library:downloaded` event carries the track id once the move is done, and `downloads.path`
/// tells where the audio is from there on. A download queued with `queue.enqueue` is not moved,
/// since the queue knows an id and the library needs the metadata.
#[tauri_groups::command(downloads)]
pub async fn enqueue(
  db: State<'_, Db>,
  api: State<'_, YtApi>,
  track: Track,
  quality: Option<AudioQuality>,
) -> Result<DownloadJob, String> {
  let id = track.id.clone();
  db.tracks().save(&track_to_db(track)).await.map_err(fail)?;
  Ok(api.enqueue(&id, quality.unwrap_or_default()))
}

/// Deletes the audio file and its record. Returns whether the track was downloaded.
#[tauri_groups::command(downloads)]
pub async fn remove(media: State<'_, MediaStore>, track_id: String) -> Result<bool, String> {
  media.remove(&track_id).await.map_err(fail)
}

/// Where the audio of the track is, ready for the asset protocol. `None` when the track is not
/// downloaded, which includes a record whose file has vanished.
#[tauri_groups::command(downloads)]
pub async fn path(
  media: State<'_, MediaStore>,
  track_id: String,
) -> Result<Option<String>, String> {
  let path = media.path(&track_id).await.map_err(fail)?;
  Ok(path.map(|p| p.to_string_lossy().into_owned()))
}

#[tauri_groups::command(downloads)]
pub async fn list(db: State<'_, Db>) -> Result<Vec<DownloadRecord>, String> {
  Ok(db.downloads().list().await.map_err(fail)?.into_iter().map(Into::into).collect())
}

/// Downloaded tracks, newest first.
#[tauri_groups::command(downloads)]
pub async fn tracks(db: State<'_, Db>, limit: u32, offset: u32) -> Result<Vec<Track>, String> {
  Ok(tracks_from_db(db.downloads().tracks(limit, offset).await.map_err(fail)?))
}

#[tauri_groups::command(downloads)]
pub async fn total_size(db: State<'_, Db>) -> Result<u64, String> {
  db.downloads().total_size().await.map_err(fail)
}

#[tauri_groups::command(downloads)]
pub async fn count(db: State<'_, Db>) -> Result<u64, String> {
  db.downloads().count().await.map_err(fail)
}
