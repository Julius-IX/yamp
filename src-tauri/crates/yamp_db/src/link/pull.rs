use cloud_kit::Log;

use super::{apply::apply, blocking, doc::Batch, Link};
use crate::{Error, Result};

/// Log entries read per request round. Bounds the memory a pull takes.
const PAGE: usize = 16;

/// Whether entries between `cursor` and the first one that is left were deleted.
pub(crate) fn has_gap(trimmed_through: Option<&str>, cursor: Option<&str>) -> bool {
  trimmed_through.is_some_and(|t| cursor.is_none_or(|c| c < t))
}

impl Link {
  /// Applies what the other devices have logged since the last pull and returns how many
  /// entities changed here.
  ///
  /// Fails with [`Error::Gap`] when a device cleaned up entries that this one has not
  /// read, `sync` recovers from that on its own.
  pub async fn pull(&self) -> Result<usize> {
    let mut applied = 0;
    for peer in self.peers().await? {
      applied += self.pull_from(&peer).await?;
    }
    if applied > 0 {
      log::info!("pulled {applied} changes");
    }
    Ok(applied)
  }

  async fn pull_from(&self, peer: &str) -> Result<usize> {
    let log = self.log_of(peer)?;
    let mut cursor = self.cursors().await?.remove(peer);
    let trimmed = self.meta_of(peer).await?.and_then(|m| m.trimmed_through);
    if has_gap(trimmed.as_deref(), cursor.as_deref()) {
      return Err(Error::Gap(peer.into()));
    }

    let mut applied = 0;
    loop {
      let (l, c) = (log.clone(), cursor.clone());
      let page = blocking(move || read_page(&l, c.as_deref())).await?;
      let last_page = page.len() < PAGE;
      for (name, batch) in page {
        let changes = batch.map(|b| b.changes).unwrap_or_default();
        applied += apply(&self.pool, &self.device, &changes, &[(peer, &name)]).await?;
        cursor = Some(name);
      }
      if last_page {
        return Ok(applied);
      }
    }
  }
}

/// The next entries with their content. An entry that cannot be parsed, or that was
/// deleted after it was listed, comes back empty. It still moves the cursor, otherwise one
/// damaged file would stop this device from ever reading past it.
fn read_page(log: &Log, cursor: Option<&str>) -> cloud_kit::Result<Vec<(String, Option<Batch>)>> {
  let mut out = Vec::new();
  for name in log.names_after(cursor, Some(PAGE))? {
    match log.read::<Batch>(&name) {
      Ok(batch) => out.push((name, Some(batch))),
      Err(cloud_kit::Error::Json(e)) => {
        log::warn!("skipping unreadable log entry {name}: {e}");
        out.push((name, None));
      }
      Err(e) if e.is_not_found() => out.push((name, None)),
      Err(e) => return Err(e),
    }
  }
  Ok(out)
}

#[cfg(test)]
mod tests {
  use super::has_gap;

  #[test]
  fn a_gap_is_cleaned_up_entries_that_were_not_read() {
    assert!(!has_gap(None, None));
    assert!(!has_gap(None, Some("b")));
    assert!(has_gap(Some("b"), None));
    assert!(has_gap(Some("b"), Some("a")));
    assert!(!has_gap(Some("b"), Some("b")));
    assert!(!has_gap(Some("b"), Some("c")));
  }
}
