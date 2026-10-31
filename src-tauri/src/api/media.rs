use super::err;
use tauri::State;
use yt_api::{AudioQuality, Playlist, StreamInfo, Track, YtApi};

#[tauri_groups::command(media)]
pub async fn search(
  api: State<'_, YtApi>,
  query: String,
  limit: Option<u32>,
) -> Result<Vec<Track>, String> {
  api.search(&query, limit.map(|l| l as usize)).await.map_err(err)
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
) -> Result<StreamInfo, String> {
  api.stream(&id, quality.unwrap_or_default()).await.map_err(err)
}
