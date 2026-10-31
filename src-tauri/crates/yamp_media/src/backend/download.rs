use crate::{
  error::Result,
  model::{AudioQuality, DownloadedFile},
};
use async_trait::async_trait;
use std::{path::Path, sync::Arc};

/// Receives download progress as a percentage from 0 to 100. May be called from any thread.
pub type ProgressSink = Arc<dyn Fn(f32) + Send + Sync>;

#[async_trait]
pub trait Downloader: Send + Sync {
  /// Saves the audio into `dir` as `<id>.<ext>`. An existing file is reused rather than
  /// fetched again.
  async fn download(
    &self,
    id: &str,
    quality: AudioQuality,
    dir: &Path,
    progress: Option<ProgressSink>,
  ) -> Result<DownloadedFile>;
}
