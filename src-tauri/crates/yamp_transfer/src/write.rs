//! The files that leave the app. Both use the same columns, so a file of failures can be fixed
//! by hand and imported again, and so can the export of the library. The columns are the ones
//! of no service in particular, since the library is not tied to any.
use crate::{
  entry::{Entry, Failure},
  text::write_record,
};

const HEADER: [&str; 7] = ["type", "collection", "title", "artist", "album", "duration_secs", "year"];

fn fields(entry: &Entry) -> [String; 7] {
  [
    entry.kind.as_str().to_owned(),
    entry.collection.clone().unwrap_or_default(),
    entry.title.clone(),
    entry.artist.clone(),
    entry.album.clone().unwrap_or_default(),
    entry.duration_secs.map(|d| d.to_string()).unwrap_or_default(),
    entry.year.map(|y| y.to_string()).unwrap_or_default(),
  ]
}

/// The whole library as one file, one row per song. A song that is in several playlists has
/// a row in each.
pub fn library_csv(entries: &[Entry]) -> String {
  let mut out = String::new();
  write_record(&mut out, &HEADER);
  for entry in entries {
    write_record(&mut out, &fields(entry).each_ref().map(String::as_str));
  }
  out
}

/// What an import could not place, with the reason in an extra column that the import ignores.
pub fn failures_csv(failures: &[Failure]) -> String {
  let mut out = String::new();
  let mut header = HEADER.to_vec();
  header.push("reason");
  write_record(&mut out, &header);
  for failure in failures {
    let cells = fields(&failure.entry);
    let mut row: Vec<&str> = cells.iter().map(String::as_str).collect();
    row.push(failure.reason.as_str());
    write_record(&mut out, &row);
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{Kind, Reason, parse};

  fn song(kind: Kind, collection: Option<&str>, title: &str) -> Entry {
    Entry {
      row: 0,
      kind,
      collection: collection.map(Into::into),
      title: title.into(),
      artist: "Queen, David Bowie".into(),
      album: Some("Hot Space".into()),
      duration_secs: Some(248),
      year: Some(1982),
    }
  }

  #[test]
  fn the_library_file_reads_back_as_the_same_entries() {
    let entries = vec![
      song(Kind::Like, None, "Under Pressure"),
      song(Kind::Playlist, Some("Gym, \"hard\""), "=Weird"),
    ];
    let parsed = parse(&library_csv(&entries), "x").unwrap();
    assert!(parsed.rejected.is_empty());
    let read: Vec<Entry> = parsed.entries.into_iter().map(|e| Entry { row: 0, ..e }).collect();
    assert_eq!(read, entries);
  }

  #[test]
  fn the_file_starts_with_the_header() {
    assert_eq!(library_csv(&[]), "type,collection,title,artist,album,duration_secs,year\r\n");
  }

  #[test]
  fn failures_carry_the_reason_and_can_be_imported_again() {
    let failures = vec![Failure { entry: song(Kind::Like, None, "Lost"), reason: Reason::NoMatch }];
    let text = failures_csv(&failures);
    assert!(text.starts_with("type,collection,title,artist,album,duration_secs,year,reason\r\n"));
    assert!(text.contains(",no match found\r\n"));
    let parsed = parse(&text, "x").unwrap();
    assert_eq!(parsed.entries.len(), 1);
    assert_eq!(parsed.entries[0].title, "Lost");
  }
}
