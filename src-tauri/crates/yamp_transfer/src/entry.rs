use std::fmt;

/// What an entry becomes in the library.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
  Like,
  Playlist,
  /// Yamp keeps a saved album as a playlist under the album id of the provider, so an album
  /// has to be looked up as an album and cannot be rebuilt from its songs.
  Album,
}

impl Kind {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Like => "like",
      Self::Playlist => "playlist",
      Self::Album => "album",
    }
  }

  /// Takes the words other services use for the same thing, like "Favorite" or "Liked".
  pub fn parse(word: &str) -> Option<Self> {
    let word = word.trim().to_lowercase();
    if word.contains("like") || word.contains("favo") || word.contains("loved") {
      Some(Self::Like)
    } else if word.contains("album") {
      Some(Self::Album)
    } else if word.contains("playlist") {
      Some(Self::Playlist)
    } else {
      None
    }
  }
}

/// The service a file came from, as far as its columns tell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
  Spotify,
  SoundCloud,
  /// A file that Yamp wrote itself.
  Yamp,
  Other,
}

impl Source {
  pub fn name(self) -> &'static str {
    match self {
      Self::Spotify => "Spotify",
      Self::SoundCloud => "SoundCloud",
      Self::Yamp => "Yamp",
      Self::Other => "CSV",
    }
  }
}

/// One row of a file, or one row to write. It is a song, or an album when `kind` says so.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
  /// Number of the record in the file, the header being 1. Zero for an entry that was not read
  /// from a file. Blank lines are not counted.
  pub row: usize,
  pub kind: Kind,
  /// The playlist or album the entry belongs to. None for a like.
  pub collection: Option<String>,
  /// Empty for an album, which is matched by `album` alone.
  pub title: String,
  /// Several artists are joined with a comma.
  pub artist: String,
  pub album: Option<String>,
  pub duration_secs: Option<u32>,
  pub year: Option<u32>,
}

/// Why an entry did not make it into the library.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
  /// The row has no title, or no album for an album.
  Unreadable,
  /// The provider answered, but nothing it returned is close enough.
  NoMatch,
  /// The provider did not answer. Another attempt can work.
  LookupFailed,
}

impl Reason {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Unreadable => "no title",
      Self::NoMatch => "no match found",
      Self::LookupFailed => "lookup failed",
    }
  }
}

impl fmt::Display for Reason {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(self.as_str())
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Failure {
  pub entry: Entry,
  pub reason: Reason,
}
