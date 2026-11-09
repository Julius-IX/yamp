//! Keeps one request alive at a time, the newest.
//!
//! Playback waits for a single stream URL, so when another track is picked before the first
//! lookup has answered, that lookup is of no use anymore. It is stopped and not left to finish,
//! since a lookup is a whole yt-dlp run and the older ones would only compete with the newest
//! for bandwidth and cpu.
use crate::error::{MediaError, Result};
use std::{
  future::Future,
  sync::{Mutex, PoisonError},
};
use tokio::task::AbortHandle;

#[derive(Default)]
pub struct Latest {
  current: Mutex<Option<AbortHandle>>,
}

impl Latest {
  pub fn new() -> Self {
    Self::default()
  }

  /// Runs `work` and stops the one that was started before it. The stopped caller gets
  /// [`MediaError::Cancelled`]. The work runs as a task of its own because a future that is
  /// dropped is the only way to end it, and its owner is not around to do that. Dropping the
  /// future also drops the process it waits for, which `kill_on_drop` turns into a kill.
  pub async fn run<T, F>(&self, work: F) -> Result<T>
  where
    T: Send + 'static,
    F: Future<Output = Result<T>> + Send + 'static,
  {
    let task = tokio::spawn(work);
    let older = self.current.lock().unwrap_or_else(PoisonError::into_inner).replace(task.abort_handle());
    if let Some(older) = older {
      older.abort();
    }
    match task.await {
      Ok(result) => result,
      Err(e) if e.is_cancelled() => Err(MediaError::Cancelled),
      Err(e) => Err(MediaError::Engine(format!("the lookup stopped: {e}"))),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::{sync::Arc, time::Duration};

  #[tokio::test]
  async fn a_newer_request_stops_the_one_before_it() {
    let latest = Arc::new(Latest::new());
    let first = {
      let latest = latest.clone();
      tokio::spawn(async move {
        latest
          .run(async {
            tokio::time::sleep(Duration::from_secs(30)).await;
            Ok(1)
          })
          .await
      })
    };
    // let the first request register before the second one arrives
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(latest.run(async { Ok(2) }).await.unwrap(), 2);
    assert!(matches!(first.await.unwrap(), Err(MediaError::Cancelled)));
  }

  #[tokio::test]
  async fn a_request_that_finished_is_not_affected_by_the_next() {
    let latest = Latest::new();
    assert_eq!(latest.run(async { Ok(1) }).await.unwrap(), 1);
    assert_eq!(latest.run(async { Ok(2) }).await.unwrap(), 2);
  }

  #[tokio::test]
  async fn a_failure_is_passed_on_as_it_is() {
    let latest = Latest::new();
    let out = latest.run(async { Err::<(), _>(MediaError::NotFound) }).await;
    assert!(matches!(out, Err(MediaError::NotFound)));
  }
}
