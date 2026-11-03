use super::{
  fail,
  wire::{LibraryPlaylist, tracks_from_db, tracks_to_db},
};
use tauri::State;
use yamp_db::Db;
use yt_api::Track;

/// Creates an empty playlist. The id is random, so playlists made on different devices never
/// collide once they are linked.
#[tauri_groups::command(playlists)]
pub async fn create(
  db: State<'_, Db>,
  name: String,
  description: Option<String>,
) -> Result<LibraryPlaylist, String> {
  Ok(db.playlists().create(&name, description.as_deref()).await.map_err(fail)?.into())
}

/// Replaces the name and the description.
#[tauri_groups::command(playlists)]
pub async fn update(
  db: State<'_, Db>,
  id: String,
  name: String,
  description: Option<String>,
) -> Result<(), String> {
  db.playlists().update(&id, &name, description.as_deref()).await.map_err(fail)
}

/// Returns whether the playlist existed. Its tracks stay in the library.
#[tauri_groups::command(playlists)]
pub async fn delete(db: State<'_, Db>, id: String) -> Result<bool, String> {
  db.playlists().delete(&id).await.map_err(fail)
}

#[tauri_groups::command(playlists)]
pub async fn get(db: State<'_, Db>, id: String) -> Result<Option<LibraryPlaylist>, String> {
  Ok(db.playlists().get(&id).await.map_err(fail)?.map(Into::into))
}

/// Most recently changed first.
#[tauri_groups::command(playlists)]
pub async fn list(db: State<'_, Db>) -> Result<Vec<LibraryPlaylist>, String> {
  Ok(db.playlists().list().await.map_err(fail)?.into_iter().map(Into::into).collect())
}

/// In playlist order.
#[tauri_groups::command(playlists)]
pub async fn tracks(db: State<'_, Db>, id: String) -> Result<Vec<Track>, String> {
  Ok(tracks_from_db(db.playlists().tracks(&id).await.map_err(fail)?))
}

/// Appends to the end. A track that is already in the list is added again.
#[tauri_groups::command(playlists)]
pub async fn add_tracks(db: State<'_, Db>, id: String, tracks: Vec<Track>) -> Result<(), String> {
  db.playlists().add_tracks(&id, &tracks_to_db(tracks)).await.map_err(fail)
}

/// Positions count from 0. The same track can be in a list twice, so an entry is named by its
/// position and not by its track id.
#[tauri_groups::command(playlists)]
pub async fn remove_at(db: State<'_, Db>, id: String, position: u32) -> Result<(), String> {
  db.playlists().remove_at(&id, position).await.map_err(fail)
}

/// Moves the entry at `from` so that it ends up at `to`.
#[tauri_groups::command(playlists)]
pub async fn move_track(db: State<'_, Db>, id: String, from: u32, to: u32) -> Result<(), String> {
  db.playlists().move_track(&id, from, to).await.map_err(fail)
}
