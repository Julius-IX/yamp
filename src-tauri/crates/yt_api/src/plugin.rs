use crate::{api::YtApi, platform};
use std::sync::Arc;
use tauri::{
  plugin::{Builder, TauriPlugin},
  Emitter, Manager, Wry,
};
use yamp_media::{Backend, DownloadQueue};

/// Event carrying a `DownloadJob` snapshot whenever a queued download changes.
pub const DOWNLOAD_EVENT: &str = "yt-api:download";

/// Two at a time hides the startup cost of each yt-dlp call without hammering the provider.
const PARALLEL_DOWNLOADS: usize = 2;

/// Registers the backend and the queue. Add it to the builder before anything uses [`YtApi`].
pub fn init() -> TauriPlugin<Wry> {
  Builder::new("yt-api")
    .setup(|app, api| {
      let data_dir = app.path().app_data_dir()?;
      let backend = platform::backend(&api, &data_dir)?;

      let emitter = app.clone();
      let queue = DownloadQueue::new(
        backend.downloader.clone(),
        // app private storage avoids storage permissions on Android
        data_dir.join("music"),
        PARALLEL_DOWNLOADS,
        tauri::async_runtime::handle().inner().clone(),
        Arc::new(move |job| {
          let _ = emitter.emit(DOWNLOAD_EVENT, job);
        }),
      );
      install_if_missing(backend.clone());
      app.manage(YtApi::new(backend, queue));
      log::info!("yt-api ready");
      Ok(())
    })
    .build()
}

/// Fetches the tooling on first launch so search works without a manual step. Runs in the
/// background, a failure only surfaces when something needs the tooling.
fn install_if_missing(backend: Backend) {
  tauri::async_runtime::spawn(async move {
    match backend.updater.status().await {
      Ok(s) if !s.installed => {
        if let Err(e) = backend.updater.update().await {
          log::error!("yt-dlp install failed: {e}");
        }
      }
      Err(e) => log::error!("yt-dlp check failed: {e}"),
      Ok(_) => {}
    }
  });
}
