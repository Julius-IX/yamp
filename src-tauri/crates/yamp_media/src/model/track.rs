use serde::Serialize;
use ts_types::TsType;

/// A playable item. Only `id` and `title` are guaranteed, the rest is best effort because
/// providers differ in what their listings expose.
#[derive(Debug, Clone, Serialize, TsType)]
#[serde(rename_all = "camelCase")]
pub struct Track {
  pub id: String,
  pub title: String,
  pub artist: String,
  pub album: Option<String>,
  pub duration_secs: Option<f64>,
  pub thumbnail: Option<String>,
  pub release_year: Option<u32>,
}

#[derive(Debug, Clone, Serialize, TsType)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
  pub id: String,
  pub title: String,
  pub tracks: Vec<Track>,
}
