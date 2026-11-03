use super::{
  fail,
  wire::{track_to_db, tracks_from_db},
};
use tauri::State;
use yamp_db::Db;
use yt_api::Track;

/// The metadata of the track is saved with the like, so the track can be listed later without
/// asking the provider again.
#[tauri_groups::command(likes)]
pub async fn like(db: State<'_, Db>, track: Track) -> Result<(), String> {
  db.likes().like(&track_to_db(track)).await.map_err(fail)
}

/// Returns whether the track was liked before.
#[tauri_groups::command(likes)]
pub async fn unlike(db: State<'_, Db>, track_id: String) -> Result<bool, String> {
  db.likes().unlike(&track_id).await.map_err(fail)
}

#[tauri_groups::command(likes)]
pub async fn is_liked(db: State<'_, Db>, track_id: String) -> Result<bool, String> {
  db.likes().is_liked(&track_id).await.map_err(fail)
}

/// Every liked id, newest first. Marks the hearts of a list without a query per row.
#[tauri_groups::command(likes)]
pub async fn ids(db: State<'_, Db>) -> Result<Vec<String>, String> {
  db.likes().ids().await.map_err(fail)
}

/// Newest first. A like whose track has not arrived from another device yet is skipped.
#[tauri_groups::command(likes)]
pub async fn list(db: State<'_, Db>, limit: u32, offset: u32) -> Result<Vec<Track>, String> {
  Ok(tracks_from_db(db.likes().list(limit, offset).await.map_err(fail)?))
}

#[tauri_groups::command(likes)]
pub async fn count(db: State<'_, Db>) -> Result<u64, String> {
  db.likes().count().await.map_err(fail)
}
