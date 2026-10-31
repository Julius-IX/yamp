//! yt-dlp backed implementation of the media traits.
//!
//! [`YtDlpBackend`] holds all provider logic (which URLs to query, which flags to pass, how to
//! read the output) and delegates only process execution to a [`YtDlpEngine`]. A platform adds
//! support by implementing that one trait.
mod backend;
mod engine;
mod parse;

#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
mod desktop;
#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
pub use desktop::DesktopEngine;

pub use backend::YtDlpBackend;
pub use engine::{Opt, YtDlpArgs, YtDlpEngine};
