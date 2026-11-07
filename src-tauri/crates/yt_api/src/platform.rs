//! Picks the backend for the platform the app runs on. This is the only place that knows which
//! engine exists where, and which catalog browses it, the rest of the crate and the app only see a `Backend`.
use std::{error::Error, path::Path, sync::Arc};
use tauri::{plugin::PluginApi, Wry};
use yamp_media::{ytdlp::YtDlpBackend, Backend};
use ytm_catalog::YtMusicCatalog;

/// yt-dlp is a plain binary here, kept in an app owned directory so updates never touch the system.
#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
pub fn backend(_api: &PluginApi<Wry, ()>, data_dir: &Path) -> Result<Backend, Box<dyn Error>> {
  let engine = yamp_media::ytdlp::DesktopEngine::new(data_dir.join("bin"));
  Ok(Backend::single(YtDlpBackend::new(engine)).with_catalog(Arc::new(YtMusicCatalog::new())))
}

/// yt-dlp runs as an embedded Python build behind a Kotlin plugin, there is no binary to manage.
#[cfg(target_os = "android")]
pub fn backend(api: &PluginApi<Wry, ()>, _data_dir: &Path) -> Result<Backend, Box<dyn Error>> {
  let engine = crate::android::engine(api)?;
  Ok(Backend::single(YtDlpBackend::new(engine)).with_catalog(Arc::new(YtMusicCatalog::new())))
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos", target_os = "android")))]
compile_error!("no media backend for this platform yet");
