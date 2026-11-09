//! Tells when something changed below a folder.
//!
//! Copying an album into a folder makes hundreds of events, and a scan wants to run once after
//! them. Events are therefore held back until the folder has been quiet for a moment, and what
//! comes out is the watched folder, not the single files. The scan walks the folder anyway.
//!
//! Some file systems send no events for changes made by other programs, shared storage on Android
//! is one. A caller that needs to cover those scans again at other cues, such as the app coming
//! back to the front.

use std::{
  path::{Path, PathBuf},
  sync::{Arc, Mutex, PoisonError},
  time::Duration,
};

use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{DebounceEventResult, Debouncer, new_debouncer};

pub struct FolderWatch {
  debouncer: Mutex<Debouncer<RecommendedWatcher>>,
  roots: Arc<Mutex<Vec<PathBuf>>>,
}

impl FolderWatch {
  /// `on_change` gets a watched folder once its events have settled for `quiet`. It runs on the
  /// thread of the watcher, so it should hand the work on and return.
  pub fn new(quiet: Duration, on_change: impl Fn(&Path) + Send + 'static) -> notify::Result<Self> {
    let roots: Arc<Mutex<Vec<PathBuf>>> = Arc::default();
    let seen = roots.clone();
    let debouncer = new_debouncer(quiet, move |result: DebounceEventResult| {
      let events = match result {
        Ok(events) => events,
        Err(e) => {
          log::warn!("folder watch: {e}");
          return;
        }
      };
      let roots = seen.lock().unwrap_or_else(PoisonError::into_inner);
      // one call per folder, however many of its files changed
      let mut hit: Vec<&PathBuf> = Vec::new();
      for event in &events {
        if let Some(root) = roots.iter().find(|r| event.path.starts_with(r)) {
          if !hit.contains(&root) {
            hit.push(root);
          }
        }
      }
      for root in hit {
        on_change(root);
      }
    })?;
    Ok(Self { debouncer: Mutex::new(debouncer), roots })
  }

  /// Starts watching a folder and everything below it.
  pub fn watch(&self, dir: &Path) -> notify::Result<()> {
    self.debouncer
      .lock()
      .unwrap_or_else(PoisonError::into_inner)
      .watcher()
      .watch(dir, RecursiveMode::Recursive)?;
    let mut roots = self.roots.lock().unwrap_or_else(PoisonError::into_inner);
    if !roots.iter().any(|r| r == dir) {
      roots.push(dir.to_path_buf());
    }
    Ok(())
  }

  pub fn unwatch(&self, dir: &Path) {
    self.roots.lock().unwrap_or_else(PoisonError::into_inner).retain(|r| r != dir);
    // a folder that is gone from disk cannot be unwatched, and there is nothing left to do then
    let _ = self.debouncer.lock().unwrap_or_else(PoisonError::into_inner).watcher().unwatch(dir);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::sync::mpsc::channel;

  /// The app keeps the watch in shared state, which needs both.
  #[test]
  fn can_be_shared_between_threads() {
    fn shared<T: Send + Sync>() {}
    shared::<FolderWatch>();
  }

  #[test]
  fn reports_the_folder_once_after_a_burst() {
    let dir = std::env::temp_dir().join(format!("folder-watch-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("album")).unwrap();
    let (tx, rx) = channel();
    let watch = FolderWatch::new(Duration::from_millis(300), move |root| {
      let _ = tx.send(root.to_path_buf());
    })
    .unwrap();
    watch.watch(&dir).unwrap();

    for n in 0..5 {
      std::fs::write(dir.join("album").join(format!("{n}.mp3")), b"x").unwrap();
    }
    assert_eq!(rx.recv_timeout(Duration::from_secs(10)).unwrap(), dir);
    // the burst was one report
    assert!(rx.recv_timeout(Duration::from_millis(1500)).is_err());

    watch.unwatch(&dir);
    std::fs::write(dir.join("later.mp3"), b"x").unwrap();
    assert!(rx.recv_timeout(Duration::from_millis(1500)).is_err());
    let _ = std::fs::remove_dir_all(dir);
  }
}
