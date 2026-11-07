//! Small conversions between how YouTube Music words things and what the app stores.
use yamp_media::Track;
use ytmapi_rs::{common::Thumbnail, parse::ParsedSongArtist};

/// "3:45" or "1:02:03" as seconds. Anything else, like an empty string, is unknown.
pub fn secs(text: &str) -> Option<f64> {
  let parts = text
    .trim()
    .split(':')
    .map(|p| p.trim().parse::<u32>().ok())
    .collect::<Option<Vec<_>>>()?;
  if parts.is_empty() || parts.len() > 3 {
    return None;
  }
  Some(parts.iter().fold(0u32, |total, p| total * 60 + p) as f64)
}

/// The first four digit number in the text, which is how years are worded ("2026", "Single • 2026").
pub fn year(text: &str) -> Option<u32> {
  text
    .split(|c: char| !c.is_ascii_digit())
    .find(|part| part.len() == 4)
    .and_then(|part| part.parse().ok())
}

/// The largest picture of the list.
pub fn best_thumbnail(thumbnails: &[Thumbnail]) -> Option<String> {
  thumbnails.iter().max_by_key(|t| t.width * t.height).map(|t| t.url.clone())
}

pub fn artist_names(artists: &[ParsedSongArtist]) -> String {
  artists.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", ")
}

/// A listing with no thumbnail still has the frame the video player shows, which for songs is
/// the cover. This is the same fallback the yt-dlp backend uses.
fn thumbnail_or_frame(id: &str, thumbnails: &[Thumbnail]) -> Option<String> {
  best_thumbnail(thumbnails).or_else(|| Some(format!("https://i.ytimg.com/vi/{id}/mqdefault.jpg")))
}

pub struct TrackParts<'a> {
  pub id: &'a str,
  pub title: String,
  pub artist: String,
  pub album: Option<String>,
  /// As written by the provider, like "3:45".
  pub duration: &'a str,
  pub thumbnails: &'a [Thumbnail],
  pub release_year: Option<u32>,
}

pub fn track(p: TrackParts) -> Track {
  Track {
    thumbnail: thumbnail_or_frame(p.id, p.thumbnails),
    duration_secs: secs(p.duration),
    album: p.album.filter(|a| !a.is_empty()),
    release_year: p.release_year,
    id: p.id.to_owned(),
    title: p.title,
    artist: p.artist,
  }
}

/// Ids end up in requests, so only the characters YouTube itself uses are accepted.
pub fn valid_id(id: &str) -> bool {
  !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn durations_are_read_as_seconds() {
    assert_eq!(secs("3:45"), Some(225.0));
    assert_eq!(secs("1:02:03"), Some(3723.0));
    assert_eq!(secs(" 0:09 "), Some(9.0));
    assert_eq!(secs(""), None);
    assert_eq!(secs("live"), None);
    assert_eq!(secs("1:2:3:4"), None);
  }

  #[test]
  fn years_are_found_in_longer_text() {
    assert_eq!(year("2026"), Some(2026));
    assert_eq!(year("Single • 2019"), Some(2019));
    assert_eq!(year("12 songs"), None);
    assert_eq!(year(""), None);
  }

  #[test]
  fn ids_with_url_characters_are_rejected() {
    assert!(valid_id("UCabc-_123"));
    assert!(!valid_id(""));
    assert!(!valid_id("a/b"));
    assert!(!valid_id("a?b=c"));
  }

  #[test]
  fn a_track_without_a_cover_gets_the_video_frame() {
    let t = track(TrackParts {
      id: "aaaaaaaaaaa",
      title: "Song".into(),
      artist: "Band".into(),
      album: Some(String::new()),
      duration: "3:00",
      thumbnails: &[],
      release_year: None,
    });
    assert_eq!(t.thumbnail.as_deref(), Some("https://i.ytimg.com/vi/aaaaaaaaaaa/mqdefault.jpg"));
    assert_eq!(t.duration_secs, Some(180.0));
    assert_eq!(t.album, None);
  }
}
