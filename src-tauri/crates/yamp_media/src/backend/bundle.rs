use super::{Catalog, Downloader, MediaSource, NoCatalog, Updater};
use crate::{error::Result, model::Track};
use std::{collections::HashSet, sync::Arc};

/// The four parts the app works with. They may come from one provider or from different ones.
#[derive(Clone)]
pub struct Backend {
  pub source: Arc<dyn MediaSource>,
  pub downloader: Arc<dyn Downloader>,
  pub updater: Arc<dyn Updater>,
  pub catalog: Arc<dyn Catalog>,
}

impl Backend {
  /// For a provider that implements source, downloader and updater itself. It has no catalog
  /// until one is added with [`Backend::with_catalog`].
  pub fn single<T: MediaSource + Downloader + Updater + 'static>(provider: T) -> Self {
    let provider = Arc::new(provider);
    Self {
      source: provider.clone(),
      downloader: provider.clone(),
      updater: provider,
      catalog: Arc::new(NoCatalog),
    }
  }

  pub fn with_catalog(mut self, catalog: Arc<dyn Catalog>) -> Self {
    self.catalog = catalog;
    self
  }

  /// Music videos from the catalog together with the other videos of the source. The catalog only
  /// knows music videos, so a plain video like a talk or a long mix only turns up through the
  /// source. Both are asked at once, and one of them failing still leaves the other one's list.
  pub async fn search_videos(&self, query: &str, limit: usize) -> Result<Vec<Track>> {
    let (music, plain) = tokio::join!(self.catalog.search_videos(query, limit), self.source.search_videos(query, limit));
    match (music, plain) {
      (Ok(music), Ok(plain)) => Ok(merged(music, plain, limit)),
      (Ok(only), Err(e)) | (Err(e), Ok(only)) => {
        log::warn!("one of the video searches failed: {e}");
        Ok(merged(only, vec![], limit))
      }
      (Err(e), Err(_)) => Err(e),
    }
  }
}

/// Takes from both lists in turns, so the cut at `limit` leaves some of each. Either one alone
/// can fill the whole limit, and the other would then never show. A video that both list is kept
/// once, at its first place.
fn merged(first: Vec<Track>, second: Vec<Track>, limit: usize) -> Vec<Track> {
  let (mut first, mut second) = (first.into_iter(), second.into_iter());
  let (mut seen, mut out) = (HashSet::new(), Vec::new());
  while out.len() < limit {
    let (a, b) = (first.next(), second.next());
    if a.is_none() && b.is_none() {
      break;
    }
    for track in a.into_iter().chain(b) {
      if out.len() < limit && seen.insert(track.id.clone()) {
        out.push(track);
      }
    }
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  fn track(id: &str) -> Track {
    Track {
      id: id.into(),
      title: id.into(),
      artist: String::new(),
      album: None,
      duration_secs: None,
      thumbnail: None,
      release_year: None,
    }
  }

  fn ids(tracks: &[Track]) -> Vec<&str> {
    tracks.iter().map(|t| t.id.as_str()).collect()
  }

  #[test]
  fn a_video_in_both_lists_appears_once() {
    let all = merged(vec![track("a"), track("b")], vec![track("b"), track("c")], 10);
    assert_eq!(ids(&all), ["a", "b", "c"]);
  }

  #[test]
  fn the_result_is_cut_to_the_limit() {
    let all = merged(vec![track("a"), track("b")], vec![track("c")], 2);
    assert_eq!(ids(&all), ["a", "c"]);
  }

  #[test]
  fn a_long_first_list_leaves_room_for_the_second() {
    let music: Vec<Track> = (0..20).map(|n| track(&format!("m{n}"))).collect();
    let all = merged(music, vec![track("x"), track("y")], 6);
    assert_eq!(ids(&all), ["m0", "x", "m1", "y", "m2", "m3"]);
  }
}
