use super::{
  fail,
  wire::{Album, SavedAlbum, tracks_from_db, tracks_to_db},
};
use tauri::State;
use yamp_db::Db;
use yt_api::Track;

/// The tracks are saved in the given order, which is the order the album is played in. Saving
/// an album again replaces its track list and keeps the original save time.
#[tauri_groups::command(albums)]
pub async fn save(db: State<'_, Db>, album: Album, tracks: Vec<Track>) -> Result<(), String> {
  db.albums().save(&album.into(), &tracks_to_db(tracks)).await.map_err(fail)
}

/// Returns whether the album was saved. Its tracks stay in the library, they may be liked or in
/// a playlist.
#[tauri_groups::command(albums)]
pub async fn remove(db: State<'_, Db>, id: String) -> Result<bool, String> {
  db.albums().remove(&id).await.map_err(fail)
}

#[tauri_groups::command(albums)]
pub async fn is_saved(db: State<'_, Db>, id: String) -> Result<bool, String> {
  db.albums().is_saved(&id).await.map_err(fail)
}

#[tauri_groups::command(albums)]
pub async fn get(db: State<'_, Db>, id: String) -> Result<Option<SavedAlbum>, String> {
  Ok(db.albums().get(&id).await.map_err(fail)?.map(Into::into))
}

#[tauri_groups::command(albums)]
pub async fn list(db: State<'_, Db>) -> Result<Vec<SavedAlbum>, String> {
  Ok(db.albums().list().await.map_err(fail)?.into_iter().map(Into::into).collect())
}

#[tauri_groups::command(albums)]
pub async fn tracks(db: State<'_, Db>, id: String) -> Result<Vec<Track>, String> {
  Ok(tracks_from_db(db.albums().tracks(&id).await.map_err(fail)?))
}
