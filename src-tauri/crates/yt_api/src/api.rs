use yamp_media::{
  AudioQuality, Backend, DownloadJob, DownloadQueue, EngineStatus, Playlist, Result, StreamInfo,
  Track,
};

const DEFAULT_LIMIT: usize = 25;

pub struct YtApi {
  backend: Backend,
  queue: DownloadQueue,
}

/// Failures are logged here once, so every caller gets the same trace without repeating it.
fn logged<T>(op: &str, result: Result<T>) -> Result<T> {
  if let Err(e) = &result {
    log::warn!("{op} failed: {e}");
  }
  result
}

impl YtApi {
  pub(crate) fn new(backend: Backend, queue: DownloadQueue) -> Self {
    Self { backend, queue }
  }

  // SECTION: reading from the provider

  /// `limit` defaults to a page that fits a result list.
  pub async fn search(&self, query: &str, limit: Option<usize>) -> Result<Vec<Track>> {
    logged("search", self.backend.source.search(query, limit.unwrap_or(DEFAULT_LIMIT)).await)
  }

  pub async fn track(&self, id: &str) -> Result<Track> {
    logged("track lookup", self.backend.source.track(id).await)
  }

  pub async fn playlist(&self, id: &str) -> Result<Playlist> {
    logged("playlist lookup", self.backend.source.playlist(id).await)
  }

  /// Tracks that fit after `id`, for autoplay.
  pub async fn related(&self, id: &str, limit: Option<usize>) -> Result<Vec<Track>> {
    logged("related lookup", self.backend.source.related(id, limit.unwrap_or(DEFAULT_LIMIT)).await)
  }

  /// A playable URL that expires, so it should be resolved right before playback.
  pub async fn stream(&self, id: &str, quality: AudioQuality) -> Result<StreamInfo> {
    logged("stream lookup", self.backend.source.stream(id, quality).await)
  }

  // SECTION: download queue

  /// Returns at once. Progress arrives through [`DOWNLOAD_EVENT`].
  pub fn enqueue(&self, id: &str, quality: AudioQuality) -> DownloadJob {
    self.queue.enqueue(id, quality)
  }

  /// Only a job that has not started can be cancelled.
  pub fn cancel(&self, id: &str) -> bool {
    self.queue.cancel(id)
  }

  pub fn jobs(&self) -> Vec<DownloadJob> {
    self.queue.list()
  }

  pub fn clear_finished(&self) {
    self.queue.clear_finished();
  }

  // SECTION: provider tooling

  pub async fn status(&self) -> Result<EngineStatus> {
    logged("engine status", self.backend.updater.status().await)
  }

  /// Installs the tooling when missing, otherwise updates it. Provider sites change often, so
  /// this is the fix when searches or streams start failing.
  pub async fn update(&self) -> Result<EngineStatus> {
    logged("engine update", self.backend.updater.update().await)
  }
}
