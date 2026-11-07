use super::Track;
use serde::{Deserialize, Serialize};
use ts_types::TsType;

/// What a collection in a result list is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TsType)]
#[serde(rename_all = "lowercase")]
pub enum ItemKind {
  Artist,
  Album,
  Playlist,
}

/// An artist, album or playlist as a listing shows it. Only `id` and `title` are guaranteed.
/// `label` is the provider's own word for it, like "Single", and is shown as given.
#[derive(Debug, Clone, Serialize, TsType)]
#[serde(rename_all = "camelCase")]
pub struct Item {
  pub id: String,
  pub kind: ItemKind,
  pub title: String,
  pub subtitle: Option<String>,
  pub label: Option<String>,
  pub thumbnail: Option<String>,
  pub release_year: Option<u32>,
}

/// A track from an artist's top songs. `plays` is display text, since providers round it.
#[derive(Debug, Clone, Serialize, TsType)]
#[serde(rename_all = "camelCase")]
pub struct TopSong {
  pub track: Track,
  pub plays: Option<String>,
}

/// An artist page. A provider fills what it has, so everything past the name may be empty.
/// `audience` is display text, like "2.1M subscribers", and `thumbnail` is the wide header picture.
#[derive(Debug, Clone, Serialize, TsType)]
#[serde(rename_all = "camelCase")]
pub struct ArtistPage {
  pub id: String,
  pub name: String,
  pub description: Option<String>,
  pub audience: Option<String>,
  pub thumbnail: Option<String>,
  pub top_songs: Vec<TopSong>,
  pub albums: Vec<Item>,
  pub singles: Vec<Item>,
  pub videos: Vec<Track>,
  pub related: Vec<Item>,
}

/// An album with its tracks. Tracks carry the album's artist and cover, as listings of an
/// album often leave them out.
#[derive(Debug, Clone, Serialize, TsType)]
#[serde(rename_all = "camelCase")]
pub struct AlbumPage {
  pub id: String,
  pub title: String,
  pub artist: String,
  pub year: Option<u32>,
  /// Album, Single or EP, as the provider words it.
  pub release_type: Option<String>,
  pub thumbnail: Option<String>,
  pub tracks: Vec<Track>,
}
