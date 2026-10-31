use crate::{
  error::Result,
  model::{AudioQuality, Playlist, StreamInfo, Track},
};
use async_trait::async_trait;

/// Everything that reads from a provider without touching disk.
///
/// Ids are opaque to callers. Each implementation validates the ids it is given.
#[async_trait]
pub trait MediaSource: Send + Sync {
  async fn search(&self, query: &str, limit: usize) -> Result<Vec<Track>>;
  async fn track(&self, id: &str) -> Result<Track>;
  async fn playlist(&self, id: &str) -> Result<Playlist>;
  /// Tracks that continue well after `id`, used for autoplay and radio. Excludes `id` itself.
  async fn related(&self, id: &str, limit: usize) -> Result<Vec<Track>>;
  async fn stream(&self, id: &str, quality: AudioQuality) -> Result<StreamInfo>;
}
