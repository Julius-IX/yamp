use super::{
  fail,
  wire::{Artist, FollowedArtist},
};
use tauri::State;
use yamp_db::Db;

/// Following again refreshes the name and picture and keeps the original follow time.
#[tauri_groups::command(artists)]
pub async fn follow(db: State<'_, Db>, artist: Artist) -> Result<(), String> {
  db.artists().follow(&artist.into()).await.map_err(fail)
}

/// Returns whether the artist was followed before.
#[tauri_groups::command(artists)]
pub async fn unfollow(db: State<'_, Db>, id: String) -> Result<bool, String> {
  db.artists().unfollow(&id).await.map_err(fail)
}

#[tauri_groups::command(artists)]
pub async fn is_following(db: State<'_, Db>, id: String) -> Result<bool, String> {
  db.artists().is_following(&id).await.map_err(fail)
}

#[tauri_groups::command(artists)]
pub async fn list(db: State<'_, Db>) -> Result<Vec<FollowedArtist>, String> {
  Ok(db.artists().list().await.map_err(fail)?.into_iter().map(Into::into).collect())
}

#[tauri_groups::command(artists)]
pub async fn count(db: State<'_, Db>) -> Result<u64, String> {
  db.artists().count().await.map_err(fail)
}
