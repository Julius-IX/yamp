use yamp_media::{
  AlbumPage, ArtistPage, AudioQuality, Backend, DownloadJob, DownloadQueue, EngineStatus, Explore,
  Item, ItemKind, Latest, MediaError, Playlist, Result, StreamInfo, Track,
};

const DEFAULT_LIMIT: usize = 25;

pub struct YtApi {
  backend: Backend,
  queue: DownloadQueue,
  /// The lookup of the track that is wanted now, see [`YtApi::stream_now`].
  playing: Latest,
}

/// Failures are logged here once, so every caller gets the same trace without repeating it.
fn logged<T>(op: &str, result: Result<T>) -> Result<T> {
  match &result {
    // a newer lookup took over on purpose, so there is nothing to report
    Err(MediaError::Cancelled) => log::debug!("{op} cancelled"),
    Err(e) => log::warn!("{op} failed: {e}"),
    Ok(_) => {}
  }
  result
}

impl YtApi {
  pub(crate) fn new(backend: Backend, queue: DownloadQueue) -> Self {
    Self { backend, queue, playing: Latest::new() }
  }

  // SECTION: reading from the provider

  /// `limit` defaults to a page that fits a result list. The catalog lists songs with their
  /// artist and cover, the stream extractor is the fallback when the catalog is out of reach.
  pub async fn search(&self, query: &str, limit: Option<usize>) -> Result<Vec<Track>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT);
    match self.backend.catalog.search_songs(query, limit).await {
      Ok(tracks) => Ok(tracks),
      Err(e) => {
        log::warn!("catalog search failed, using the engine: {e}");
        logged("search", self.backend.source.search(query, limit).await)
      }
    }
  }

  pub async fn search_videos(&self, query: &str, limit: Option<usize>) -> Result<Vec<Track>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT);
    logged("video search", self.backend.catalog.search_videos(query, limit).await)
  }

  /// Artists, albums or playlists, one kind at a time because providers list them apart.
  pub async fn search_items(
    &self,
    query: &str,
    kind: ItemKind,
    limit: Option<usize>,
  ) -> Result<Vec<Item>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT);
    logged("item search", self.backend.catalog.search_items(query, kind, limit).await)
  }

  pub async fn artist(&self, id: &str) -> Result<ArtistPage> {
    logged("artist lookup", self.backend.catalog.artist(id).await)
  }

  pub async fn album(&self, id: &str) -> Result<AlbumPage> {
    logged("album lookup", self.backend.catalog.album(id).await)
  }

  /// New releases, trending songs, new videos and the moods to browse by.
  pub async fn explore(&self) -> Result<Explore> {
    logged("explore lookup", self.backend.catalog.explore().await)
  }

  /// The chart of artists for a country, or worldwide without one.
  pub async fn top_artists(&self, country: Option<&str>) -> Result<Vec<Item>> {
    logged("top artists lookup", self.backend.catalog.top_artists(country).await)
  }

  /// The playlists of a mood. `params` is what [`yamp_media::Mood`] carries.
  pub async fn mood_playlists(&self, params: &str) -> Result<Vec<Item>> {
    logged("mood lookup", self.backend.catalog.mood_playlists(params).await)
  }

  pub async fn track(&self, id: &str) -> Result<Track> {
    logged("track lookup", self.backend.source.track(id).await)
  }

  pub async fn playlist(&self, id: &str) -> Result<Playlist> {
    match self.backend.catalog.playlist(id).await {
      Ok(playlist) => Ok(playlist),
      Err(e) => {
        log::warn!("catalog playlist lookup failed, using the engine: {e}");
        logged("playlist lookup", self.backend.source.playlist(id).await)
      }
    }
  }

  /// Tracks that fit after `id`, for autoplay.
  pub async fn related(&self, id: &str, limit: Option<usize>) -> Result<Vec<Track>> {
    logged("related lookup", self.backend.source.related(id, limit.unwrap_or(DEFAULT_LIMIT)).await)
  }

  /// A playable URL that expires, so it should be resolved right before playback.
  pub async fn stream(&self, id: &str, quality: AudioQuality) -> Result<StreamInfo> {
    logged("stream lookup", self.backend.source.stream(id, quality).await)
  }

  /// Like [`YtApi::stream`] for the track that is wanted right now. A newer call stops the one
  /// still under way, which then fails with [`MediaError::Cancelled`]. Playback only ever waits
  /// for the newest one. Lookups ahead of time go through `stream`, those must not be stopped by
  /// a track that is picked and must not stop it either.
  pub async fn stream_now(&self, id: &str, quality: AudioQuality) -> Result<StreamInfo> {
    let source = self.backend.source.clone();
    let id = id.to_owned();
    logged("stream lookup", self.playing.run(async move { source.stream(&id, quality).await }).await)
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
