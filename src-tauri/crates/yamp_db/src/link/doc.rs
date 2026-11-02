//! The wire format. These types are what ends up in the cloud folder, so they are kept
//! apart from the models of the stores and change only in ways older versions can read.
//! Unknown fields are ignored on read and optional fields may be missing.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One entity at one point in time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Change {
  /// A plain string and not `Kind`, so a kind added by a newer app does not make the
  /// whole batch unreadable. Such a change is skipped on apply.
  pub kind: String,
  pub key: String,
  /// When the entity was changed, by the clock of the device that did it.
  pub ts_ms: i64,
  /// The device that made the change. It breaks ties between equal times.
  pub device: String,
  /// The entity as one of the `*Doc` types below. Absent when it was deleted.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub doc: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TrackDoc {
  pub title: String,
  pub artist: String,
  pub album: Option<String>,
  pub duration_secs: Option<f64>,
  pub thumbnail: Option<String>,
  pub release_year: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LikeDoc {
  pub liked_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ArtistDoc {
  pub name: String,
  pub thumbnail: Option<String>,
  pub followed_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AlbumDoc {
  pub title: String,
  pub artist: String,
  pub cover: Option<String>,
  pub release_year: Option<u32>,
  pub added_ms: i64,
  pub tracks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PlaylistDoc {
  pub name: String,
  pub description: Option<String>,
  pub created_ms: i64,
  pub updated_ms: i64,
  pub tracks: Vec<String>,
}
