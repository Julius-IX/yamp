//! Turns yt-dlp JSON into the shared model. Kept free of I/O so it can be tested with
//! captured output.
use crate::{
  error::{MediaError, Result},
  model::{Playlist, StreamInfo, Track},
};
use serde_json::Value;

/// Livestreams, podcasts, mixes and shorts fall outside this range. Applied only when a
/// duration is known, since flat YouTube Music results often carry none.
const MIN_TRACK_SECS: f64 = 45.0;
const MAX_TRACK_SECS: f64 = 900.0;

// SECTION: field helpers
fn str_of(v: &Value, k: &str) -> Option<String> {
  v.get(k).and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_owned)
}

fn artist_of(v: &Value) -> String {
  str_of(v, "artist")
    .or_else(|| {
      let names: Vec<&str> =
        v.get("artists")?.as_array()?.iter().filter_map(Value::as_str).collect();
      (!names.is_empty()).then(|| names.join(", "))
    })
    .or_else(|| str_of(v, "channel"))
    .or_else(|| str_of(v, "uploader"))
    .map(|a| a.strip_suffix(" - Topic").map(str::to_owned).unwrap_or(a))
    .unwrap_or_default()
}

fn thumbnail_of(v: &Value, id: &str) -> String {
  // Thumbnails are ordered worst to best. Flat YouTube Music results often have none, so
  // the CDN URL is the fallback.
  v.get("thumbnails")
    .and_then(Value::as_array)
    .and_then(|a| a.iter().rev().find_map(|t| t.get("url").and_then(Value::as_str)))
    .map(str::to_owned)
    .or_else(|| str_of(v, "thumbnail"))
    .unwrap_or_else(|| format!("https://i.ytimg.com/vi/{id}/mqdefault.jpg"))
}

fn track_of(v: &Value) -> Option<Track> {
  let id = str_of(v, "id")?;
  // `track` is the clean song name on music uploads, `title` often carries the artist too.
  let title = str_of(v, "track").or_else(|| str_of(v, "title"))?;
  Some(Track {
    artist: artist_of(v),
    album: str_of(v, "album"),
    duration_secs: v.get("duration").and_then(Value::as_f64),
    thumbnail: Some(thumbnail_of(v, &id)),
    release_year: v.get("release_year").and_then(Value::as_u64).and_then(|y| u32::try_from(y).ok()),
    id,
    title,
  })
}

fn first_json(out: &str) -> Result<Value> {
  let line = out.lines().find(|l| !l.trim().is_empty()).ok_or(MediaError::NotFound)?;
  Ok(serde_json::from_str(line)?)
}

// SECTION: parsers per command output
/// One JSON object per line, as printed by `--flat-playlist --dump-json`.
pub fn entries(out: &str, music_length_only: bool) -> Vec<Track> {
  out
    .lines()
    .filter_map(|l| serde_json::from_str::<Value>(l).ok())
    .filter(|v| match v.get("duration").and_then(Value::as_f64) {
      Some(d) if music_length_only => (MIN_TRACK_SECS..=MAX_TRACK_SECS).contains(&d),
      _ => true,
    })
    .filter_map(|v| track_of(&v))
    .collect()
}

pub fn track(out: &str) -> Result<Track> {
  track_of(&first_json(out)?).ok_or(MediaError::NotFound)
}

/// A single object, as printed by `--flat-playlist --dump-single-json`.
pub fn playlist(out: &str) -> Result<Playlist> {
  let v = first_json(out)?;
  let tracks = v
    .get("entries")
    .and_then(Value::as_array)
    .map(|a| a.iter().filter_map(track_of).collect())
    .unwrap_or_default();
  Ok(Playlist {
    id: str_of(&v, "id").ok_or(MediaError::NotFound)?,
    title: str_of(&v, "title").unwrap_or_default(),
    tracks,
  })
}

pub fn stream(out: &str) -> Result<StreamInfo> {
  let v = first_json(out)?;
  let url = str_of(&v, "url").ok_or(MediaError::NotFound)?;
  let ext = str_of(&v, "ext").unwrap_or_else(|| "m4a".into());
  Ok(StreamInfo {
    mime: mime_of(&ext).into(),
    bitrate_kbps: v.get("abr").and_then(Value::as_f64),
    expires_at: expire_param(&url),
    url,
    ext,
  })
}

fn mime_of(ext: &str) -> &'static str {
  match ext {
    "m4a" | "mp4" => "audio/mp4",
    "webm" => "audio/webm",
    "opus" | "ogg" => "audio/ogg",
    "mp3" => "audio/mpeg",
    _ => "application/octet-stream",
  }
}

/// googlevideo URLs carry their own deadline as an `expire` query parameter.
fn expire_param(url: &str) -> Option<u64> {
  let query = url.split_once('?')?.1;
  query.split('&').find_map(|kv| kv.strip_prefix("expire=")?.parse().ok())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn flat_entries_filter_length_and_fill_gaps() {
    let out = concat!(
      r#"{"id":"aaaaaaaaaaa","title":"Song","channel":"Band - Topic","duration":200.0}"#,
      "\n",
      r#"{"id":"bbbbbbbbbbb","title":"Live 24/7","channel":"x","duration":99999.0}"#,
      "\n",
      r#"{"id":"ccccccccccc","title":"No duration","uploader":"y"}"#,
      "\n",
      "not json\n",
    );
    let t = entries(out, true);
    assert_eq!(t.len(), 2);
    assert_eq!(t[0].artist, "Band");
    assert_eq!(t[0].thumbnail.as_deref(), Some("https://i.ytimg.com/vi/aaaaaaaaaaa/mqdefault.jpg"));
    assert_eq!(t[1].id, "ccccccccccc");
    assert_eq!(entries(out, false).len(), 3);
  }

  #[test]
  fn music_metadata_wins_over_upload_title() {
    let out = r#"{"id":"a","title":"Band - Song (Official)","track":"Song","artist":"Band","album":"LP","release_year":2019,"thumbnails":[{"url":"s"},{"url":"big"}]}"#;
    let t = track(out).unwrap();
    assert_eq!((t.title.as_str(), t.artist.as_str()), ("Song", "Band"));
    assert_eq!(t.album.as_deref(), Some("LP"));
    assert_eq!(t.release_year, Some(2019));
    assert_eq!(t.thumbnail.as_deref(), Some("big"));
  }

  #[test]
  fn playlist_skips_unavailable_entries() {
    let out = r#"{"id":"PL1","title":"Mix","entries":[null,{"id":"a","title":"A"},{"title":"no id"}]}"#;
    let p = playlist(out).unwrap();
    assert_eq!((p.id.as_str(), p.tracks.len()), ("PL1", 1));
  }

  #[test]
  fn stream_reads_format_and_deadline() {
    let out = r#"{"url":"https://r1.googlevideo.com/videoplayback?expire=1790000000&x=1","ext":"m4a","abr":129.5}"#;
    let s = stream(out).unwrap();
    assert_eq!(s.expires_at, Some(1_790_000_000));
    assert_eq!(s.mime, "audio/mp4");
    assert_eq!(s.bitrate_kbps, Some(129.5));
  }

  #[test]
  fn empty_output_is_not_found() {
    assert!(matches!(track(""), Err(MediaError::NotFound)));
    assert!(matches!(stream("\n"), Err(MediaError::NotFound)));
  }
}
