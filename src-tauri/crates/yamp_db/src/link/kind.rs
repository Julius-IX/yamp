/// The entities that are shared between devices.
///
/// The names are part of the wire format and of the database, so they never change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
  Track,
  Like,
  Artist,
  Playlist,
  /// The cover of a playlist, keyed by the id of the playlist.
  Cover,
  /// A key value pair of `synced_values`, see `store::value`.
  Value,
}

impl Kind {
  pub const ALL: [Kind; 6] = [Kind::Track, Kind::Like, Kind::Artist, Kind::Playlist, Kind::Cover, Kind::Value];

  pub fn as_str(self) -> &'static str {
    match self {
      Kind::Track => "track",
      Kind::Like => "like",
      Kind::Artist => "artist",
      Kind::Playlist => "playlist",
      Kind::Cover => "cover",
      Kind::Value => "value",
    }
  }

  /// `None` for a kind this version does not know, which can come from a newer app.
  pub fn parse(s: &str) -> Option<Kind> {
    Kind::ALL.into_iter().find(|k| k.as_str() == s)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn names_roundtrip() {
    for kind in Kind::ALL {
      assert_eq!(Kind::parse(kind.as_str()), Some(kind));
    }
    assert_eq!(Kind::parse("podcast"), None);
  }
}
