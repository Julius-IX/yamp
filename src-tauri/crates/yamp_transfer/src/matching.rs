//! Decides which result of the provider is the song or album of an entry. The same song is
//! worded differently by every service ("(feat. X)", "- Remastered 2011", other spellings of the
//! artist), so titles and artists are compared after cleaning and by how much they overlap. A
//! wrong match is worse than none, since it ends up in the library unnoticed, while a miss shows
//! up in the failure report. The limits therefore lean to refusing.
use crate::entry::{Entry, Kind};

/// A result of the provider, reduced to what the comparison uses. For an album the artist is
/// whatever line of text the provider shows next to it.
#[derive(Debug, Clone)]
pub struct Candidate {
  pub title: String,
  pub artist: String,
  pub duration_secs: Option<u32>,
}

const MIN_TITLE: f64 = 0.75;
const MIN_ARTIST: f64 = 0.6;
const MIN_SCORE: f64 = 0.8;
/// Wider than this and the result is another version of the song, like a live or extended one.
const MAX_DURATION_GAP: u32 = 12;

/// The text to search the provider for.
pub fn query(entry: &Entry) -> String {
  let wanted = if entry.kind == Kind::Album { entry.album.as_deref().unwrap_or("") } else { &entry.title };
  let artist = artist_names(&entry.artist).into_iter().next().unwrap_or_default();
  // the brackets are dropped since they hold what differs between services and narrow the search
  format!("{artist} {}", strip_brackets(wanted)).trim().to_owned()
}

/// The index of the best candidate, or None when none is close enough. Of equal candidates the
/// first wins, since the provider lists the likeliest first.
pub fn pick(entry: &Entry, candidates: &[Candidate]) -> Option<usize> {
  let album = entry.kind == Kind::Album;
  let wanted = if album { entry.album.as_deref().unwrap_or("") } else { &entry.title };
  let wanted = clean(wanted);
  let artists = artist_names(&entry.artist);

  let mut best: Option<(usize, f64)> = None;
  for (i, c) in candidates.iter().enumerate() {
    let title = dice(&wanted, &clean(&c.title));
    if title < MIN_TITLE {
      continue;
    }
    // without an artist the title has to carry the match alone
    let artist = if artists.is_empty() { 1.0 } else { artist_similarity(&artists, &c.artist) };
    if artist < MIN_ARTIST || (artists.is_empty() && title < 0.95) {
      continue;
    }
    let gap = match (album, entry.duration_secs, c.duration_secs) {
      (false, Some(a), Some(b)) => a.abs_diff(b),
      _ => 0,
    };
    if gap > MAX_DURATION_GAP {
      continue;
    }
    let score = 0.6 * title + 0.4 * artist - f64::from(gap) / 200.0;
    if score >= MIN_SCORE && best.is_none_or(|(_, top)| score > top) {
      best = Some((i, score));
    }
  }
  best.map(|(i, _)| i)
}

fn artist_similarity(wanted: &[String], found: &str) -> f64 {
  let found = artist_names(found);
  wanted.iter().flat_map(|w| found.iter().map(move |f| dice(w, f))).fold(0.0, f64::max)
}

/// The artists of a text that lists them, cleaned. Services join them with commas, ampersands,
/// "feat." or the bullet that the provider puts between the parts of a subtitle.
fn artist_names(value: &str) -> Vec<String> {
  let mut text = format!(" {} ", value.to_lowercase());
  for word in [" feat. ", " feat ", " ft. ", " ft ", " featuring ", " with ", " and ", " x "] {
    text = text.replace(word, ",");
  }
  text
    .split([',', ';', '&', '/', '+', '•', '·'])
    .map(clean)
    .filter(|a| !a.is_empty())
    .collect()
}

fn strip_brackets(text: &str) -> String {
  let mut out = String::with_capacity(text.len());
  let mut depth = 0u32;
  for c in text.chars() {
    match c {
      '(' | '[' => depth += 1,
      ')' | ']' => depth = depth.saturating_sub(1),
      _ if depth == 0 => out.push(c),
      _ => {}
    }
  }
  out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Lowercase letters and digits separated by single spaces. What a service adds after a dash
/// is cut off when it names a version ("- Remastered 2011", "- Radio Edit"), a dash that is part
/// of the title is kept.
fn clean(text: &str) -> String {
  let mut text = strip_brackets(&text.to_lowercase());
  if let Some((head, tail)) = text.split_once(" - ") {
    const VERSIONS: [&str; 8] = ["remaster", "version", "edit", "mix", "live", "mono", "stereo", "feat"];
    if VERSIONS.iter().any(|v| tail.contains(v)) {
      text = head.to_owned();
    }
  }
  let letters: String = text.chars().map(|c| if c.is_alphanumeric() { c } else { ' ' }).collect();
  letters.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Sørensen–Dice on pairs of letters, 1 for equal texts. It tolerates a typo or a dropped word
/// and does not care about the order, which suits titles that only differ in small ways.
fn dice(a: &str, b: &str) -> f64 {
  if a == b {
    return 1.0;
  }
  let pairs = |s: &str| -> Vec<(char, char)> {
    let chars: Vec<char> = s.chars().collect();
    chars.windows(2).map(|w| (w[0], w[1])).collect()
  };
  let (mut left, right) = (pairs(a), pairs(b));
  if left.is_empty() || right.is_empty() {
    return 0.0;
  }
  let total = left.len() + right.len();
  let mut shared = 0;
  for pair in right {
    if let Some(at) = left.iter().position(|p| *p == pair) {
      left.swap_remove(at);
      shared += 1;
    }
  }
  2.0 * shared as f64 / total as f64
}

#[cfg(test)]
mod tests {
  use super::*;

  fn entry(title: &str, artist: &str, secs: Option<u32>) -> Entry {
    Entry {
      row: 2,
      kind: Kind::Like,
      collection: None,
      title: title.into(),
      artist: artist.into(),
      album: None,
      duration_secs: secs,
      year: None,
    }
  }

  fn found(title: &str, artist: &str, secs: Option<u32>) -> Candidate {
    Candidate { title: title.into(), artist: artist.into(), duration_secs: secs }
  }

  #[test]
  fn matches_across_the_wording_of_services() {
    let wanted = entry("Under Pressure - Remastered 2011", "Queen, David Bowie", Some(248));
    let results = [
      found("Under Pressure (Live)", "Someone Else", Some(250)),
      found("Under Pressure", "Queen & David Bowie", Some(249)),
    ];
    assert_eq!(pick(&wanted, &results), Some(1));
  }

  #[test]
  fn a_feature_credit_does_not_matter() {
    let wanted = entry("Stay (feat. Justin Bieber)", "The Kid LAROI", Some(141));
    assert_eq!(pick(&wanted, &[found("Stay", "The Kid LAROI, Justin Bieber", Some(141))]), Some(0));
  }

  #[test]
  fn refuses_the_wrong_artist() {
    let wanted = entry("Yesterday", "The Beatles", Some(125));
    assert_eq!(pick(&wanted, &[found("Yesterday", "Boyz II Men", Some(125))]), None);
  }

  #[test]
  fn refuses_another_version_by_its_length() {
    let wanted = entry("Time", "Pink Floyd", Some(413));
    assert_eq!(pick(&wanted, &[found("Time", "Pink Floyd", Some(540))]), None);
  }

  #[test]
  fn refuses_another_song() {
    let wanted = entry("Time", "Pink Floyd", None);
    assert_eq!(pick(&wanted, &[found("Money", "Pink Floyd", None)]), None);
  }

  #[test]
  fn an_unknown_length_does_not_block_a_match() {
    let wanted = entry("Time", "Pink Floyd", None);
    assert_eq!(pick(&wanted, &[found("Time", "Pink Floyd", Some(413))]), Some(0));
  }

  #[test]
  fn without_an_artist_only_an_exact_title_is_taken() {
    let wanted = entry("Glue", "", None);
    assert_eq!(pick(&wanted, &[found("Glue", "Bicep", None)]), Some(0));
    assert_eq!(pick(&wanted, &[found("Glued", "Bicep", None)]), None);
  }

  #[test]
  fn the_first_of_equal_candidates_wins() {
    let wanted = entry("Time", "Pink Floyd", None);
    let results = [found("Time", "Pink Floyd", None), found("Time", "Pink Floyd", None)];
    assert_eq!(pick(&wanted, &results), Some(0));
  }

  #[test]
  fn albums_are_matched_by_name_and_the_subtitle() {
    let mut wanted = entry("", "Queen", None);
    wanted.kind = Kind::Album;
    wanted.album = Some("Hot Space".into());
    let results = [found("Hot Space", "Album • Queen • 1982", None), found("Hot Spaces", "Album • Other • 2001", None)];
    assert_eq!(pick(&wanted, &results), Some(0));
    assert_eq!(pick(&wanted, &results[1..]), None);
  }

  #[test]
  fn the_query_names_the_first_artist_and_drops_brackets() {
    let wanted = entry("Stay (feat. Justin Bieber)", "The Kid LAROI, Justin Bieber", None);
    assert_eq!(query(&wanted), "the kid laroi Stay");
  }
}
