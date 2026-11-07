use crate::{
  error::{MediaError, Result},
  model::{AlbumPage, ArtistPage, Item, ItemKind, Playlist, Track},
};
use async_trait::async_trait;

/// Browsing a provider's catalog. Everything here is metadata, nothing is played from it.
///
/// The shapes are the app's own. An implementation maps whatever its provider returns into
/// them, so a provider changing its API only touches that implementation. Ids are opaque to
/// callers and each implementation validates the ones it is given.
#[async_trait]
pub trait Catalog: Send + Sync {
  async fn search_songs(&self, query: &str, limit: usize) -> Result<Vec<Track>>;
  /// Music videos. Unlike songs they are not held to a track length.
  async fn search_videos(&self, query: &str, limit: usize) -> Result<Vec<Track>>;
  /// Artists, albums or playlists, one kind at a time because providers list them apart.
  async fn search_items(&self, query: &str, kind: ItemKind, limit: usize) -> Result<Vec<Item>>;
  async fn artist(&self, id: &str) -> Result<ArtistPage>;
  async fn album(&self, id: &str) -> Result<AlbumPage>;
  async fn playlist(&self, id: &str) -> Result<Playlist>;
}

/// For a provider with no catalog of its own. Every call is unsupported, so callers fall back.
pub struct NoCatalog;

#[async_trait]
impl Catalog for NoCatalog {
  async fn search_songs(&self, _: &str, _: usize) -> Result<Vec<Track>> {
    Err(MediaError::Unsupported)
  }
  async fn search_videos(&self, _: &str, _: usize) -> Result<Vec<Track>> {
    Err(MediaError::Unsupported)
  }
  async fn search_items(&self, _: &str, _: ItemKind, _: usize) -> Result<Vec<Item>> {
    Err(MediaError::Unsupported)
  }
  async fn artist(&self, _: &str) -> Result<ArtistPage> {
    Err(MediaError::Unsupported)
  }
  async fn album(&self, _: &str) -> Result<AlbumPage> {
    Err(MediaError::Unsupported)
  }
  async fn playlist(&self, _: &str) -> Result<Playlist> {
    Err(MediaError::Unsupported)
  }
}
