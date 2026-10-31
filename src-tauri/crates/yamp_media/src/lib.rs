//! Backend-agnostic music source interfaces.
//!
//! The player talks to a [`Backend`] and never to a concrete provider. A backend is made of
//! three narrow traits, so a provider can be assembled from parts, for example a metadata-only
//! API paired with a generic HTTP downloader.
//!
//! - [`MediaSource`]: search, metadata, playlists, related tracks and playable stream URLs.
//! - [`Downloader`]: saving a track to disk with progress.
//! - [`Updater`]: keeping the provider's own tooling current.
//!
//! This crate has no Tauri dependency on purpose. It can be checked and tested alone, and the
//! platform glue lives in the crate that depends on it.

mod backend;
mod error;
mod model;
mod queue;
pub mod ytdlp;

pub use backend::{Backend, Downloader, MediaSource, ProgressSink, Updater};
pub use error::{MediaError, Result};
pub use model::{
  AudioQuality, DownloadJob, DownloadedFile, EngineStatus, JobState, Playlist, StreamInfo, Track,
};
pub use queue::{DownloadQueue, JobSink};
