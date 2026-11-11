//! The data export that Spotify itself offers on the privacy page of an account, which is a ZIP
//! of JSON files. Two of them hold the library:
//!
//! - `YourLibrary.json` has the liked songs under `tracks` and the saved albums under `albums`.
//! - `Playlist1.json` has the playlists with their songs. A large account gets more files, like
//!   `Playlist2.json`, and each can be imported on its own.
//!
//! The files are read leniently, since Spotify has changed their fields before and a field that
//! is gone should only lose that detail. Followed artists, podcasts and audiobooks are not part
//! of what Yamp stores for a library, so they are skipped without a mention.
use std::collections::HashSet;

use serde_json::Value;

use crate::{
  entry::{Entry, Failure, Kind, Reason, Source},
  read::{Parsed, ReadError},
};

pub(crate) fn looks_like_json(text: &str) -> bool {
  matches!(text.trim_start_matches('\u{feff}').trim_start().chars().next(), Some('{' | '['))
}

pub(crate) fn parse(text: &str, name: &str) -> Result<Parsed, ReadError> {
  let value: Value =
    serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| ReadError::Json(e.to_string()))?;
  if is_history(&value) {
    return Err(ReadError::History);
  }
  let mut parsed = Parsed { source: Source::Spotify, entries: Vec::new(), rejected: Vec::new() };
  let list = |key: &str| value.get(key).and_then(Value::as_array);
  let (playlists, likes, albums) = (list("playlists"), list("tracks"), list("albums"));
  if playlists.is_none() && likes.is_none() && albums.is_none() {
    return Err(ReadError::Unrecognised);
  }

  for playlist in playlists.into_iter().flatten() {
    let collection = text_of(playlist, "name").unwrap_or_else(|| name.trim().to_owned());
    let collection = if collection.is_empty() { "Imported".to_owned() } else { collection };
    for (i, item) in playlist.get("items").and_then(Value::as_array).into_iter().flatten().enumerate() {
      // an episode or an audiobook has no track in its place
      let Some(track) = item.get("track").filter(|t| t.is_object()) else { continue };
      let entry = entry(i + 1, Kind::Playlist, Some(&collection), text_of(track, "trackName"), track, "artistName", "albumName");
      push(&mut parsed, entry);
    }
  }

  for (i, track) in likes.into_iter().flatten().enumerate() {
    push(&mut parsed, entry(i + 1, Kind::Like, None, text_of(track, "track"), track, "artist", "album"));
  }

  let mut seen = HashSet::new();
  for (i, item) in albums.into_iter().flatten().enumerate() {
    let (album, artist) = (text_of(item, "album"), text_of(item, "artist").unwrap_or_default());
    let entry = Entry {
      row: i + 1,
      kind: Kind::Album,
      collection: album.clone(),
      title: String::new(),
      artist: artist.clone(),
      album,
      duration_secs: None,
      year: None,
    };
    if entry.album.is_none() {
      parsed.rejected.push(Failure { entry, reason: Reason::Unreadable });
    } else if seen.insert((entry.album.clone(), artist)) {
      parsed.entries.push(entry);
    }
  }
  Ok(parsed)
}

fn text_of(value: &Value, key: &str) -> Option<String> {
  value.get(key).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned)
}

/// Spotify gives no length for a song in these files, so the match has only the names to go by.
fn entry(
  row: usize,
  kind: Kind,
  collection: Option<&str>,
  title: Option<String>,
  source: &Value,
  artist_key: &str,
  album_key: &str,
) -> (Entry, bool) {
  let titled = title.is_some();
  let entry = Entry {
    row,
    kind,
    collection: collection.map(str::to_owned),
    title: title.unwrap_or_default(),
    artist: text_of(source, artist_key).unwrap_or_default(),
    album: text_of(source, album_key),
    duration_secs: None,
    year: None,
  };
  (entry, titled)
}

fn push(parsed: &mut Parsed, (entry, titled): (Entry, bool)) {
  if titled {
    parsed.entries.push(entry);
  } else {
    parsed.rejected.push(Failure { entry, reason: Reason::Unreadable });
  }
}

/// The files of the streaming history are a list of plays, which has nothing to import.
fn is_history(value: &Value) -> bool {
  let first = value.as_array().and_then(|a| a.first());
  first.is_some_and(|play| ["endTime", "ts", "master_metadata_track_name"].iter().any(|k| play.get(k).is_some()))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::parse as parse_file;

  const LIBRARY: &str = r#"{
    "tracks": [
      {"artist": "The Cranberries", "album": "No Need To Argue", "track": "Zombie", "uri": "spotify:track:1"},
      {"artist": "Nobody", "album": "", "track": "", "uri": ""}
    ],
    "albums": [
      {"artist": "Queen", "album": "Hot Space", "uri": "spotify:album:1"},
      {"artist": "Queen", "album": "Hot Space", "uri": "spotify:album:1"}
    ],
    "artists": [{"name": "Queen", "uri": "spotify:artist:1"}],
    "shows": [], "episodes": []
  }"#;

  const PLAYLISTS: &str = r#"{"playlists": [
    {"name": "Road Trip", "lastModifiedDate": "2024-01-01", "items": [
      {"track": {"trackName": "Time", "artistName": "Pink Floyd", "albumName": "The Dark Side of the Moon", "trackUri": "spotify:track:2"}, "episode": null},
      {"track": null, "episode": {"name": "A podcast"}},
      {"track": {"trackName": "", "artistName": "Ghost"}}
    ]},
    {"name": "", "items": []}
  ]}"#;

  #[test]
  fn reads_the_liked_songs_and_albums_of_the_library_file() {
    let parsed = parse_file(LIBRARY, "YourLibrary").unwrap();
    assert_eq!(parsed.source, Source::Spotify);
    assert_eq!(parsed.entries.len(), 2);
    let like = &parsed.entries[0];
    assert_eq!((like.kind, like.title.as_str(), like.artist.as_str()), (Kind::Like, "Zombie", "The Cranberries"));
    assert_eq!((like.collection.as_deref(), like.album.as_deref()), (None, Some("No Need To Argue")));
    let album = &parsed.entries[1];
    assert_eq!((album.kind, album.album.as_deref(), album.artist.as_str()), (Kind::Album, Some("Hot Space"), "Queen"));
    assert_eq!(parsed.rejected.len(), 1);
    assert_eq!((parsed.rejected[0].entry.row, parsed.rejected[0].reason), (2, Reason::Unreadable));
  }

  #[test]
  fn reads_playlists_and_skips_what_is_not_a_song() {
    let parsed = parse_file(PLAYLISTS, "Playlist1").unwrap();
    assert_eq!(parsed.entries.len(), 1);
    let song = &parsed.entries[0];
    assert_eq!((song.kind, song.collection.as_deref()), (Kind::Playlist, Some("Road Trip")));
    assert_eq!((song.title.as_str(), song.artist.as_str(), song.row), ("Time", "Pink Floyd", 1));
    // the episode is skipped silently, the song without a title is reported at its position
    assert_eq!(parsed.rejected.len(), 1);
    assert_eq!(parsed.rejected[0].entry.row, 3);
  }

  #[test]
  fn the_listening_history_is_refused() {
    let history = r#"[{"endTime": "2024-01-01 10:00", "artistName": "A", "trackName": "B", "msPlayed": 1000}]"#;
    assert_eq!(parse_file(history, "x").unwrap_err(), ReadError::History);
    let extended = r#"[{"ts": "2024-01-01T10:00:00Z", "master_metadata_track_name": "B"}]"#;
    assert_eq!(parse_file(extended, "x").unwrap_err(), ReadError::History);
  }

  #[test]
  fn other_json_is_refused() {
    assert_eq!(parse_file(r#"{"userId": "1"}"#, "x").unwrap_err(), ReadError::Unrecognised);
    assert!(matches!(parse_file("{ nope", "x").unwrap_err(), ReadError::Json(_)));
  }

  #[test]
  fn csv_is_still_told_apart() {
    assert!(!looks_like_json("Title,Artist\nA,B\n"));
    assert!(looks_like_json("\u{feff}\n  {\"tracks\": []}"));
  }
}
