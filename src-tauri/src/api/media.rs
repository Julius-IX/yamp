use super::err;
use tauri::State;
use yt_api::{
  AlbumPage, ArtistPage, AudioQuality, Explore, Item, ItemKind, Playlist, StreamInfo, Track, YtApi,
};

#[tauri_groups::command(media)]
pub async fn search(
  api: State<'_, YtApi>,
  query: String,
  limit: Option<u32>,
) -> Result<Vec<Track>, String> {
  api.search(&query, limit.map(|l| l as usize)).await.map_err(err)
}

#[tauri_groups::command(media)]
pub async fn search_items(
  api: State<'_, YtApi>,
  query: String,
  kind: ItemKind,
  limit: Option<u32>,
) -> Result<Vec<Item>, String> {
  api.search_items(&query, kind, limit.map(|l| l as usize)).await.map_err(err)
}

#[tauri_groups::command(media)]
pub async fn search_videos(
  api: State<'_, YtApi>,
  query: String,
  limit: Option<u32>,
) -> Result<Vec<Track>, String> {
  api.search_videos(&query, limit.map(|l| l as usize)).await.map_err(err)
}

#[tauri_groups::command(media)]
pub async fn artist(api: State<'_, YtApi>, id: String) -> Result<ArtistPage, String> {
  api.artist(&id).await.map_err(err)
}

#[tauri_groups::command(media)]
pub async fn album(api: State<'_, YtApi>, id: String) -> Result<AlbumPage, String> {
  api.album(&id).await.map_err(err)
}

#[tauri_groups::command(media)]
pub async fn explore(api: State<'_, YtApi>) -> Result<Explore, String> {
  api.explore().await.map_err(err)
}

#[tauri_groups::command(media)]
pub async fn top_artists(
  api: State<'_, YtApi>,
  country: Option<String>,
) -> Result<Vec<Item>, String> {
  api.top_artists(country.as_deref()).await.map_err(err)
}

#[tauri_groups::command(media)]
pub async fn mood_playlists(api: State<'_, YtApi>, params: String) -> Result<Vec<Item>, String> {
  api.mood_playlists(&params).await.map_err(err)
}

#[tauri_groups::command(media)]
pub async fn track(api: State<'_, YtApi>, id: String) -> Result<Track, String> {
  api.track(&id).await.map_err(err)
}

#[tauri_groups::command(media)]
pub async fn playlist(api: State<'_, YtApi>, id: String) -> Result<Playlist, String> {
  api.playlist(&id).await.map_err(err)
}

#[tauri_groups::command(media)]
pub async fn related(
  api: State<'_, YtApi>,
  id: String,
  limit: Option<u32>,
) -> Result<Vec<Track>, String> {
  api.related(&id, limit.map(|l| l as usize)).await.map_err(err)
}

#[tauri_groups::command(media)]
pub async fn stream(
  api: State<'_, YtApi>,
  id: String,
  quality: Option<AudioQuality>,
  // set for the track that is about to play, a newer one stops the lookup of the one before it
  now: Option<bool>,
) -> Result<StreamInfo, String> {
  let quality = quality.unwrap_or_default();
  if now.unwrap_or(false) {
    api.stream_now(&id, quality).await.map_err(err)
  } else {
    api.stream(&id, quality).await.map_err(err)
  }
}
