//! Turns the CSV of a service into entries. Every service words its columns in its own way, so
//! a column is found by any of the names it is known under, and one that is missing is only
//! an absent detail. Only a title (or an album) is required.
use std::{collections::HashSet, fmt};

use crate::{
  entry::{Entry, Failure, Kind, Reason, Source},
  json, text,
};

/// What a file held.
#[derive(Debug)]
pub struct Parsed {
  pub source: Source,
  pub entries: Vec<Entry>,
  /// Rows that cannot be looked up at all. They are reported like any other failure.
  pub rejected: Vec<Failure>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ReadError {
  Empty,
  /// Neither a title nor an album column, which most likely means the file is not a list of songs.
  NoTitle,
  Json(String),
  /// The streaming history of Spotify is what was played, not what is in the library.
  History,
  /// Valid JSON, but not a file of Spotify's account data that lists songs.
  Unrecognised,
}

impl fmt::Display for ReadError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(match self {
      Self::Empty => "the file is empty",
      Self::NoTitle => "the file has no column with song titles",
      Self::Json(e) => return write!(f, "the file is not valid JSON: {e}"),
      Self::History => "this is the listening history, use YourLibrary.json or a Playlist file instead",
      Self::Unrecognised => "the file has no playlists, liked songs or albums in it",
    })
  }
}

impl std::error::Error for ReadError {}

/// Column names, lowercase. The first name in a list that exists in the header is used, so the
/// more specific names come first.
const TITLE: &[&str] = &["track name", "title", "track title", "song name", "song", "track", "name"];
const ARTIST: &[&str] = &[
  "artist name(s)", "artist name", "artist names", "artist(s)", "artists", "artist", "track artist", "uploader",
  "username", "user",
];
const ALBUM: &[&str] = &["album name", "album title", "album"];
const COLLECTION: &[&str] = &["playlist name", "playlist", "collection"];
const KIND: &[&str] = &["type", "kind"];
const YEAR: &[&str] = &["release date", "release year", "year"];
const DURATION_MS: &[&str] = &["duration (ms)", "track duration (ms)", "duration_ms", "duration ms"];
const DURATION: &[&str] = &["duration_secs", "duration (s)", "duration (sec)", "duration", "length", "time"];

struct Columns {
  title: Option<usize>,
  artist: Option<usize>,
  album: Option<usize>,
  collection: Option<usize>,
  kind: Option<usize>,
  year: Option<usize>,
  duration_ms: Option<usize>,
  duration: Option<usize>,
}

impl Columns {
  fn find(header: &[String]) -> Self {
    let by = |names: &[&str]| names.iter().find_map(|n| header.iter().position(|h| h == n));
    Self {
      title: by(TITLE),
      artist: by(ARTIST),
      album: by(ALBUM),
      collection: by(COLLECTION),
      kind: by(KIND),
      year: by(YEAR),
      duration_ms: by(DURATION_MS),
      duration: by(DURATION),
    }
  }
}

/// Reads a CSV file or the JSON of Spotify's account data, which is told apart by its first
/// character. `name` is the name of the file without its extension. Exportify and similar tools write one
/// file per playlist and keep the playlist name only there, so it is the fallback for rows that
/// do not name a collection themselves.
pub fn parse(text: &str, name: &str) -> Result<Parsed, ReadError> {
  if json::looks_like_json(text) {
    return json::parse(text, name);
  }
  let mut records = text::records(text).into_iter();
  let header: Vec<String> = records.next().ok_or(ReadError::Empty)?.iter().map(|h| h.trim().to_lowercase()).collect();
  let cols = Columns::find(&header);
  if cols.title.is_none() && cols.album.is_none() {
    return Err(ReadError::NoTitle);
  }
  let rows: Vec<Vec<String>> = records.collect();
  let source = detect(&header, rows.first());
  // without song titles every row can only be an album
  let default_kind = if cols.title.is_none() { Kind::Album } else { kind_of_name(name) };
  let name = name.trim();
  let name = if name.is_empty() { "Imported" } else { name };

  let mut parsed = Parsed { source, entries: Vec::new(), rejected: Vec::new() };
  // a saved album is listed with every one of its songs by some tools, it is looked up once
  let mut albums = HashSet::new();

  for (i, row) in rows.iter().enumerate() {
    let get = |col: Option<usize>| col.and_then(|c| row.get(c)).map(|v| v.trim()).filter(|v| !v.is_empty());
    if row.iter().all(|v| v.trim().is_empty()) {
      continue;
    }
    let kind = get(cols.kind).and_then(Kind::parse).unwrap_or(default_kind);
    let (title, artist) = split_title(get(cols.title).unwrap_or(""), &artists(get(cols.artist).unwrap_or("")), source);
    let album = get(cols.album).map(str::to_owned);
    let entry = |kind, collection: Option<String>, title: String, album: Option<String>| Entry {
      // the header is the first record
      row: i + 2,
      kind,
      collection,
      title,
      artist: artist.clone(),
      album,
      duration_secs: duration(get(cols.duration_ms), get(cols.duration)),
      year: get(cols.year).and_then(year),
    };

    match kind {
      Kind::Album => {
        let wanted = album.or_else(|| get(cols.collection).map(str::to_owned));
        let Some(wanted) = wanted else {
          parsed.rejected.push(Failure { entry: entry(kind, None, String::new(), None), reason: Reason::Unreadable });
          continue;
        };
        if albums.insert((wanted.to_lowercase(), artist.to_lowercase())) {
          parsed.entries.push(entry(kind, Some(wanted.clone()), String::new(), Some(wanted)));
        }
      }
      _ if title.is_empty() => {
        let collection = collection(kind, get(cols.collection), name);
        parsed.rejected.push(Failure { entry: entry(kind, collection, title, album), reason: Reason::Unreadable });
      }
      _ => parsed.entries.push(entry(kind, collection(kind, get(cols.collection), name), title, album)),
    }
  }
  Ok(parsed)
}

fn collection(kind: Kind, named: Option<&str>, file: &str) -> Option<String> {
  (kind != Kind::Like).then(|| named.unwrap_or(file).to_owned())
}

/// The files of a liked songs list are named after it, and have no column that says so.
fn kind_of_name(name: &str) -> Kind {
  let name = name.trim().to_lowercase();
  let liked = ["liked", "favorite", "favourite", "loved"].iter().any(|w| name.contains(w)) || name == "likes";
  if liked { Kind::Like } else { Kind::Playlist }
}

/// Only the columns are looked at, plus one cell for a service that has no tell in its header.
fn detect(header: &[String], first: Option<&Vec<String>>) -> Source {
  let has = |h: &str| header.iter().any(|c| c == h);
  if has("type") && has("collection") && has("duration_secs") {
    Source::Yamp
  } else if header.iter().any(|c| c.contains("spotify") || c == "track uri") {
    Source::Spotify
  } else if header.iter().any(|c| c.contains("soundcloud"))
    || first.is_some_and(|row| row.iter().any(|v| v.contains("soundcloud.com")))
  {
    Source::SoundCloud
  } else {
    Source::Other
  }
}

/// Exportify separates the artists of a track with a semicolon, other tools use a comma.
fn artists(value: &str) -> String {
  value.split(';').map(str::trim).filter(|a| !a.is_empty()).collect::<Vec<_>>().join(", ")
}

/// SoundCloud has no field for the artist of an upload, the uploader is only whoever posted it,
/// and the artist is in the title as "Artist - Title". The title is split when it is the only
/// place that can hold the artist.
fn split_title(title: &str, artist: &str, source: Source) -> (String, String) {
  let splittable = source == Source::SoundCloud || (source == Source::Other && artist.is_empty());
  if let Some((left, right)) = title.split_once(" - ").filter(|_| splittable) {
    let (left, right) = (left.trim(), right.trim());
    if !left.is_empty() && !right.is_empty() {
      return (right.to_owned(), left.to_owned());
    }
  }
  (title.to_owned(), artist.to_owned())
}

/// Spotify counts milliseconds. The other tools write seconds, or minutes and seconds.
fn duration(ms: Option<&str>, other: Option<&str>) -> Option<u32> {
  if let Some(ms) = ms.and_then(|v| v.parse::<f64>().ok()) {
    return Some((ms / 1000.0).round() as u32);
  }
  let value = other?;
  let secs = if value.contains(':') {
    value.split(':').try_fold(0.0, |total, part| part.trim().parse::<f64>().ok().map(|p| total * 60.0 + p))?
  } else {
    value.parse::<f64>().ok()?
  };
  // a length of hours is unlikely for a song, a column that is named only "duration" is then in ms
  let secs = if !value.contains(':') && secs >= 10_000.0 { secs / 1000.0 } else { secs };
  (secs.is_finite() && secs >= 0.0).then(|| secs.round() as u32)
}

/// Takes the first four digits that look like a year, from "2011", "2011-05-02" or "05/02/2011".
fn year(value: &str) -> Option<u32> {
  let digits: Vec<char> = value.chars().collect();
  digits
    .windows(4)
    .filter(|w| w.iter().all(char::is_ascii_digit))
    .filter_map(|w| w.iter().collect::<String>().parse::<u32>().ok())
    .find(|y| (1900..=2100).contains(y))
}

#[cfg(test)]
mod tests {
  use super::*;

  const EXPORTIFY: &str = "Track URI,Track Name,Album Name,Artist Name(s),Release Date,Duration (ms),Popularity\n\
    spotify:track:1,Time,The Dark Side of the Moon,Pink Floyd,1973-03-01,413000,80\n\
    spotify:track:2,\"Under Pressure - Remastered 2011\",Hot Space,Queen;David Bowie,1982-05-21,248000,79\n";

  #[test]
  fn reads_an_exportify_file_named_after_its_playlist() {
    let parsed = parse(EXPORTIFY, "Road Trip").unwrap();
    assert_eq!(parsed.source, Source::Spotify);
    assert_eq!(parsed.entries.len(), 2);
    let first = &parsed.entries[0];
    assert_eq!(first.kind, Kind::Playlist);
    assert_eq!(first.collection.as_deref(), Some("Road Trip"));
    assert_eq!((first.title.as_str(), first.artist.as_str()), ("Time", "Pink Floyd"));
    assert_eq!(first.album.as_deref(), Some("The Dark Side of the Moon"));
    assert_eq!((first.duration_secs, first.year, first.row), (Some(413), Some(1973), 2));
    assert_eq!(parsed.entries[1].artist, "Queen, David Bowie");
  }

  #[test]
  fn a_file_named_liked_songs_is_the_likes() {
    let parsed = parse(EXPORTIFY, "Liked Songs").unwrap();
    assert!(parsed.entries.iter().all(|e| e.kind == Kind::Like && e.collection.is_none()));
  }

  #[test]
  fn soundcloud_takes_the_artist_from_the_title() {
    let text = "title,uploader,url,duration\nBicep - Glue,bicepmusic,https://soundcloud.com/bicep/glue,4:30\n";
    let parsed = parse(text, "likes").unwrap();
    assert_eq!(parsed.source, Source::SoundCloud);
    let entry = &parsed.entries[0];
    assert_eq!((entry.title.as_str(), entry.artist.as_str()), ("Glue", "Bicep"));
    assert_eq!((entry.kind, entry.duration_secs), (Kind::Like, Some(270)));
  }

  #[test]
  fn a_title_is_kept_whole_when_the_artist_has_its_own_column() {
    let text = "Title,Artist\nSong - Live,Someone\n";
    let parsed = parse(text, "x").unwrap();
    assert_eq!(parsed.entries[0].title, "Song - Live");
  }

  #[test]
  fn the_type_and_collection_columns_win_over_the_file_name() {
    let text = "type,collection,title,artist,album,duration_secs,year\n\
      like,,Time,Pink Floyd,,413,\n\
      playlist,Gym,Lose Yourself,Eminem,,326,2002\n";
    let parsed = parse(text, "whatever").unwrap();
    assert_eq!(parsed.source, Source::Yamp);
    assert_eq!(parsed.entries[0].kind, Kind::Like);
    assert_eq!(parsed.entries[1].collection.as_deref(), Some("Gym"));
  }

  #[test]
  fn an_album_listed_by_song_is_one_entry() {
    let text = "type,collection,title,artist,album\n\
      album,Hot Space,Dancer,Queen,Hot Space\n\
      album,Hot Space,Back Chat,Queen,Hot Space\n";
    let parsed = parse(text, "x").unwrap();
    assert_eq!(parsed.entries.len(), 1);
    let entry = &parsed.entries[0];
    assert_eq!((entry.kind, entry.title.as_str(), entry.album.as_deref()), (Kind::Album, "", Some("Hot Space")));
  }

  #[test]
  fn a_file_with_only_albums_reads_as_albums() {
    let parsed = parse("Album,Artist\nHot Space,Queen\n", "Saved").unwrap();
    assert_eq!(parsed.entries[0].kind, Kind::Album);
  }

  #[test]
  fn rows_without_a_title_are_rejected_with_their_row_number() {
    let parsed = parse("Title,Artist\nA,B\n,Nobody\n,\n", "x").unwrap();
    assert_eq!(parsed.entries.len(), 1);
    assert_eq!(parsed.rejected.len(), 1);
    assert_eq!((parsed.rejected[0].entry.row, parsed.rejected[0].reason), (3, Reason::Unreadable));
    assert_eq!(parsed.rejected[0].entry.artist, "Nobody");
  }

  #[test]
  fn files_that_are_no_song_list_are_refused() {
    assert_eq!(parse("", "x").unwrap_err(), ReadError::Empty);
    assert_eq!(parse("Foo,Bar\n1,2\n", "x").unwrap_err(), ReadError::NoTitle);
  }

  #[test]
  fn durations_come_in_several_forms() {
    assert_eq!(duration(Some("248000"), None), Some(248));
    assert_eq!(duration(None, Some("4:08")), Some(248));
    assert_eq!(duration(None, Some("1:02:03")), Some(3723));
    assert_eq!(duration(None, Some("248.4")), Some(248));
    assert_eq!(duration(None, Some("248000")), Some(248));
    assert_eq!(duration(None, Some("n/a")), None);
    assert_eq!(duration(None, None), None);
  }

  #[test]
  fn years_come_from_dates() {
    assert_eq!(year("2011"), Some(2011));
    assert_eq!(year("1982-05-21"), Some(1982));
    assert_eq!(year("05/21/1982"), Some(1982));
    assert_eq!(year("unknown"), None);
  }
}
