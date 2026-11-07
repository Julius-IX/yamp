//! Backend-agnostic music source interfaces.
//!
//! The player talks to a [`Backend`] and never to a concrete provider. A backend is made of
//! four narrow traits, so a provider can be assembled from parts, for example a metadata-only
//! API paired with a generic HTTP downloader.
//!
//! - [`MediaSource`]: search, metadata, playlists, related tracks and playable stream URLs.
//! - [`Catalog`]: browsing the provider's catalog, which is search by kind of result and the
//!   pages of artists and albums. It is apart from [`MediaSource`] because many providers split
//!   the two, a metadata API on one side and a stream extractor on the other.
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

pub use backend::{Backend, Catalog, Downloader, MediaSource, ProgressSink, Updater};
pub use error::{MediaError, Result};
pub use model::{
  AlbumPage, ArtistPage, AudioQuality, DownloadJob, DownloadedFile, EngineStatus, Item, ItemKind,
  JobState, Playlist, StreamInfo, TopSong, Track,
};
pub use queue::{DownloadQueue, JobSink};
