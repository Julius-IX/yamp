//! Progress of a queued download arrives as the `yt-api:download` event with a `DownloadJob`.
use tauri::State;
use yt_api::{AudioQuality, DownloadJob, YtApi};

#[tauri_groups::command(queue)]
pub fn enqueue(api: State<'_, YtApi>, id: String, quality: Option<AudioQuality>) -> DownloadJob {
  api.enqueue(&id, quality.unwrap_or_default())
}

/// Only a job that has not started can be cancelled.
#[tauri_groups::command(queue)]
pub fn cancel(api: State<'_, YtApi>, id: String) -> bool {
  api.cancel(&id)
}

/// History for a view that opens after downloads were already queued.
#[tauri_groups::command(queue)]
pub fn list(api: State<'_, YtApi>) -> Vec<DownloadJob> {
  api.jobs()
}

#[tauri_groups::command(queue)]
pub fn clear_finished(api: State<'_, YtApi>) {
  api.clear_finished();
}
