use super::err;
use tauri::State;
use yt_api::{EngineStatus, YtApi};

#[tauri_groups::command(engine)]
pub async fn status(api: State<'_, YtApi>) -> Result<EngineStatus, String> {
  api.status().await.map_err(err)
}

/// Installs the provider tooling when missing, otherwise updates it.
#[tauri_groups::command(engine)]
pub async fn update(api: State<'_, YtApi>) -> Result<EngineStatus, String> {
  api.update().await.map_err(err)
}
