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

/// Creates the playlist under a given id and fills it. An album saved to the library takes the
/// album id, so the album can be looked up by it and saving it again makes no second copy.
/// Returns whether a playlist was made, one that already has the id is left as it is.
#[tauri_groups::command(playlists)]
pub async fn save_as(
  db: State<'_, Db>,
  id: String,
  name: String,
  description: Option<String>,
  tracks: Vec<Track>,
) -> Result<bool, String> {
  db.playlists()
    .save_as(&id, &name, description.as_deref(), &tracks_to_db(tracks))
    .await
    .map_err(fail)
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
  let Some(playlist) = db.playlists().get(&id).await.map_err(fail)? else {
    return Ok(None);
  };
  let version = db.playlist_covers().versions().await.map_err(fail)?.get(&id).copied();
  Ok(Some(LibraryPlaylist::from(playlist).with_cover_version(version)))
}

/// Most recently changed first.
#[tauri_groups::command(playlists)]
pub async fn list(db: State<'_, Db>) -> Result<Vec<LibraryPlaylist>, String> {
  let versions = db.playlist_covers().versions().await.map_err(fail)?;
  let playlists = db.playlists().list().await.map_err(fail)?;
  Ok(
    playlists
      .into_iter()
      .map(|p| {
        let version = versions.get(&p.id).copied();
        LibraryPlaylist::from(p).with_cover_version(version)
      })
      .collect(),
  )
}

/// The cover that the user set, as a `data:` URL for an image. `None` when there is none, the
/// playlist then shows the `cover` thumbnail.
#[tauri_groups::command(playlists)]
pub async fn cover(db: State<'_, Db>, id: String) -> Result<Option<String>, String> {
  db.playlist_covers().data_url(&id).await.map_err(fail)
}

/// Sets the cover of a playlist from the bytes of an image in a common format. The middle square
/// is cut out and shrunk, so the caller can send a photo as it is. The cover is synced.
#[tauri_groups::command(playlists)]
pub async fn set_cover(db: State<'_, Db>, id: String, image: Vec<u8>) -> Result<(), String> {
  db.playlist_covers().set(&id, image).await.map_err(fail)
}

/// Goes back to the thumbnail of the first track. Returns whether there was a cover to remove.
#[tauri_groups::command(playlists)]
pub async fn remove_cover(db: State<'_, Db>, id: String) -> Result<bool, String> {
  db.playlist_covers().remove(&id).await.map_err(fail)
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
