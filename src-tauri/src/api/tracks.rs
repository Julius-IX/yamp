//! Tracks the library already knows. `media.search` asks the provider, while `tracks.search`
//! never leaves the device.
use super::{
  fail,
  wire::{track_from_db, tracks_from_db},
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
