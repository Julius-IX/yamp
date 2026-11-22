//! Tracks the library already knows. `media.search` asks the provider, while `tracks.search`
//! never leaves the device.
use super::{
  fail,
  wire::{TrackCover, track_from_db, tracks_from_db},
};
use tauri::State;
use yamp_db::Db;
use yt_api::Track;

const DEFAULT_LIMIT: u32 = 25;

#[tauri_groups::command(tracks)]
pub async fn search(
  db: State<'_, Db>,
  text: String,
  limit: Option<u32>,
) -> Result<Vec<Track>, String> {
  let found = db.tracks().search(&text, limit.unwrap_or(DEFAULT_LIMIT)).await.map_err(fail)?;
  Ok(tracks_from_db(found))
}

#[tauri_groups::command(tracks)]
pub async fn get(db: State<'_, Db>, id: String) -> Result<Option<Track>, String> {
  Ok(db.tracks().get(&id).await.map_err(fail)?.map(track_from_db))
}

/// Follows the order of `ids`. Ids that are not stored are left out.
#[tauri_groups::command(tracks)]
pub async fn get_many(db: State<'_, Db>, ids: Vec<String>) -> Result<Vec<Track>, String> {
  Ok(tracks_from_db(db.tracks().get_many(&ids).await.map_err(fail)?))
}

#[tauri_groups::command(tracks)]
pub async fn count(db: State<'_, Db>) -> Result<u64, String> {
  db.tracks().count().await.map_err(fail)
}

/// The covers that the user set for tracks. They are only on this device.
#[tauri_groups::command(tracks)]
pub async fn covers(db: State<'_, Db>) -> Result<Vec<TrackCover>, String> {
  let all = db.track_covers().all().await.map_err(fail)?;
  Ok(all.into_iter().map(|(track_id, url)| TrackCover { track_id, url }).collect())
}

/// Sets the cover of a track from the bytes of an image in a common format and returns it as a
/// `data:` URL. The middle square is cut out and shrunk, so the caller can send a photo as it is.
#[tauri_groups::command(tracks)]
pub async fn set_cover(db: State<'_, Db>, id: String, image: Vec<u8>) -> Result<String, String> {
  db.track_covers().set(&id, image).await.map_err(fail)
}

/// Goes back to the thumbnail of the track. Returns whether there was a cover to remove.
#[tauri_groups::command(tracks)]
pub async fn remove_cover(db: State<'_, Db>, id: String) -> Result<bool, String> {
  db.track_covers().remove(&id).await.map_err(fail)
}
