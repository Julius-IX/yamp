//! Library shapes as the frontend sees them. `yamp-db` does not export TypeScript, and its
//! `Track` and `Playlist` would clash by name with the provider types that `yt-api` registers.
//! Tracks therefore travel as the provider `Track`, which has the same fields, and every other
//! shape is mirrored here under a name that is still free.
use serde::{Deserialize, Serialize};
use ts_types::TsType;
use yamp_db as db;
use yt_api::Track;

pub fn track_to_db(t: Track) -> db::Track {
  db::Track {
    id: t.id,
    title: t.title,
    artist: t.artist,
    album: t.album,
    duration_secs: t.duration_secs,
    thumbnail: t.thumbnail,
    release_year: t.release_year,
  }
}

pub fn track_from_db(t: db::Track) -> Track {
  Track {
    id: t.id,
    title: t.title,
    artist: t.artist,
    album: t.album,
    duration_secs: t.duration_secs,
    thumbnail: t.thumbnail,
    release_year: t.release_year,
  }
}

pub fn tracks_to_db(tracks: Vec<Track>) -> Vec<db::Track> {
  tracks.into_iter().map(track_to_db).collect()
}

pub fn tracks_from_db(tracks: Vec<db::Track>) -> Vec<Track> {
  tracks.into_iter().map(track_from_db).collect()
}

#[derive(Serialize, Deserialize, TsType)]
#[serde(rename_all = "camelCase")]
pub struct Artist {
  pub id: String,
  pub name: String,
  pub thumbnail: Option<String>,
}

impl From<Artist> for db::Artist {
  fn from(a: Artist) -> Self {
    Self { id: a.id, name: a.name, thumbnail: a.thumbnail }
  }
}

#[derive(Serialize, TsType)]
#[serde(rename_all = "camelCase")]
pub struct FollowedArtist {
  pub id: String,
  pub name: String,
  pub thumbnail: Option<String>,
  pub followed_ms: i64,
}

impl From<db::FollowedArtist> for FollowedArtist {
  fn from(a: db::FollowedArtist) -> Self {
    Self { id: a.id, name: a.name, thumbnail: a.thumbnail, followed_ms: a.followed_ms }
  }
}

/// Named apart from the provider `Playlist`, which carries its tracks and has no owner.
#[derive(Serialize, TsType)]
#[serde(rename_all = "camelCase")]
pub struct LibraryPlaylist {
  pub id: String,
  pub name: String,
  pub description: Option<String>,
  pub track_count: u32,
  /// Thumbnail of the first track that has one.
  pub cover: Option<String>,
  /// When the cover that the user set was set, none without one. `playlists.cover` returns the
  /// image, and the time tells a reader that holds an image whether it is still the current one.
  pub cover_version: Option<i64>,
  pub created_ms: i64,
  pub updated_ms: i64,
}

impl LibraryPlaylist {
  pub fn with_cover_version(mut self, version: Option<i64>) -> Self {
    self.cover_version = version;
    self
  }
}

impl From<db::Playlist> for LibraryPlaylist {
  fn from(p: db::Playlist) -> Self {
    Self {
      id: p.id,
      name: p.name,
      description: p.description,
      track_count: p.track_count,
      cover: p.cover,
      cover_version: None,
      created_ms: p.created_ms,
      updated_ms: p.updated_ms,
    }
  }
}

#[derive(Serialize, TsType)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRecord {
  pub track_id: String,
  /// Lowercase file extension, which is the format of the audio.
  pub ext: String,
  pub size_bytes: u64,
  pub downloaded_ms: i64,
}

impl From<db::Download> for DownloadRecord {
  fn from(d: db::Download) -> Self {
    Self {
      track_id: d.track_id,
      ext: d.ext,
      size_bytes: d.size_bytes,
      downloaded_ms: d.downloaded_ms,
    }
  }
}

/// A cloud service the library can be linked to in this build.
#[derive(Serialize, TsType)]
#[serde(rename_all = "camelCase")]
pub struct SyncProvider {
  pub id: String,
  pub name: String,
}

/// Another device that shares the library, or this one.
#[derive(Serialize, TsType)]
#[serde(rename_all = "camelCase")]
pub struct SyncDevice {
  pub id: String,
  pub name: String,
  pub last_seen_ms: i64,
  pub this_device: bool,
}

impl From<db::DeviceInfo> for SyncDevice {
  fn from(d: db::DeviceInfo) -> Self {
    Self { id: d.id, name: d.name, last_seen_ms: d.last_seen_ms, this_device: d.this_device }
  }
}

/// What one sync moved.
#[derive(Serialize, TsType)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
  /// Entities received from other devices that changed this library.
  pub pulled: usize,
  /// Entities of this library that were sent.
  pub pushed: usize,
}

impl From<db::SyncReport> for SyncReport {
  fn from(r: db::SyncReport) -> Self {
    Self { pulled: r.pulled, pushed: r.pushed }
  }
}

#[derive(Default, Serialize, TsType)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
  pub connected: bool,
  /// Id of the cloud service, none for a link to a plain folder.
  pub provider: Option<String>,
  /// The account the library is linked to at the service.
  pub account: Option<String>,
  pub folder: Option<String>,
  pub device_name: Option<String>,
  pub device_id: Option<String>,
}
