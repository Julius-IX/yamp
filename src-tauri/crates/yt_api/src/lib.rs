//! Music source access for the app, behind one small surface.
//!
//! The app registers [`init`] once and then works with [`YtApi`] only. Which backend runs, how
//! it is installed and how it is kept up to date is decided here per platform. The app never names
//! a provider or an engine.
//!
//! The crate is a Tauri plugin so that it can reach the app handle by itself. It owns the
//! download queue and emits every job change as the [`DOWNLOAD_EVENT`] event.
mod api;
mod plugin;
mod platform;
#[cfg(target_os = "android")]
mod android;

pub use api::YtApi;
pub use plugin::{init, DOWNLOAD_EVENT};
pub use yamp_media::{
  AudioQuality, DownloadJob, DownloadedFile, EngineStatus, JobState, MediaError, Playlist, Result,
  StreamInfo, Track,
};
