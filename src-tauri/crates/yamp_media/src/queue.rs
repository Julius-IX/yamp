//! Download queue. Callers enqueue and return at once, a small pool of workers does the fetching.
//!
//! Each job is its own task that first waits for a permit, which gives first come first served
//! order and a cap on parallel downloads without a hand written scheduler. Parallelism is capped
//! because the provider throttles aggressive clients and a phone gains little from more.
use crate::{
  backend::{Downloader, ProgressSink},
  model::{AudioQuality, DownloadJob, JobState},
};
use std::{
  path::PathBuf,
  sync::{
    Arc, Mutex, MutexGuard,
    atomic::{AtomicU64, Ordering},
  },
};
use tokio::{runtime::Handle, sync::Semaphore};

/// Called with a snapshot after every change of a job. Runs on the thread that made the change,
/// so it must not block.
pub type JobSink = Arc<dyn Fn(&DownloadJob) + Send + Sync>;

/// A job plus what the worker needs but the frontend does not.
struct Entry {
  /// Tells two jobs for the same id apart, so a worker of a cancelled job can never pick up a
  /// later job that reuses the id.
  seq: u64,
  quality: AudioQuality,
  job: DownloadJob,
}

struct Inner {
  downloader: Arc<dyn Downloader>,
  dir: PathBuf,
  permits: Arc<Semaphore>,
  entries: Mutex<Vec<Entry>>,
  next_seq: AtomicU64,
  on_change: JobSink,
}

#[derive(Clone)]
pub struct DownloadQueue {
  inner: Arc<Inner>,
  runtime: Handle,
}

impl DownloadQueue {
  /// Files are saved into `dir`. Workers are spawned on `runtime`, so `enqueue` can be called
  /// from any thread.
  pub fn new(
    downloader: Arc<dyn Downloader>,
    dir: PathBuf,
    parallel: usize,
    runtime: Handle,
    on_change: JobSink,
  ) -> Self {
    let inner = Inner {
      downloader,
      dir,
      permits: Arc::new(Semaphore::new(parallel.max(1))),
      entries: Mutex::new(Vec::new()),
      next_seq: AtomicU64::new(1),
      on_change,
    };
    Self { inner: Arc::new(inner), runtime }
  }

  /// Adds a download and returns immediately. An id that is already queued or running returns
  /// the existing job instead of a duplicate.
  pub fn enqueue(&self, id: &str, quality: AudioQuality) -> DownloadJob {
    let (seq, job) = {
      let mut entries = self.inner.lock();
      if let Some(e) = entries.iter().find(|e| e.job.id == id && is_active(e.job.state)) {
        return e.job.clone();
      }
      // a finished job for the same id is stale once the track is requested again
      entries.retain(|e| e.job.id != id);
      let seq = self.inner.next_seq.fetch_add(1, Ordering::Relaxed);
      let job = DownloadJob {
        id: id.to_owned(),
        state: JobState::Queued,
        percent: 0.0,
        file: None,
        error: None,
      };
      entries.push(Entry { seq, quality, job: job.clone() });
      (seq, job)
    };

    log::info!("queued download of {id}");
    (self.inner.on_change)(&job);
    let inner = self.inner.clone();
    self.runtime.spawn(inner.work(seq));
    job
  }

  /// Removes a job that has not started. A running download cannot be stopped, since not
  /// every engine can interrupt its process.
  pub fn cancel(&self, id: &str) -> bool {
    let seq = self
      .inner
      .lock()
      .iter()
      .find(|e| e.job.id == id && e.job.state == JobState::Queued)
      .map(|e| e.seq);
    let cancelled = seq.is_some_and(|seq| {
      self.inner.modify(seq, |j| {
        let queued = j.state == JobState::Queued;
        if queued {
          j.state = JobState::Cancelled;
        }
        queued
      })
    });
    if cancelled {
      log::info!("cancelled queued download of {id}");
    }
    cancelled
  }

  /// All jobs in the order they were added.
  pub fn list(&self) -> Vec<DownloadJob> {
    self.inner.lock().iter().map(|e| e.job.clone()).collect()
  }

  /// Forgets jobs that are done, failed or cancelled. Emits nothing, a consumer is expected to
  /// drop them from its own list when it asks for this.
  pub fn clear_finished(&self) {
    self.inner.lock().retain(|e| is_active(e.job.state));
  }
}

fn is_active(state: JobState) -> bool {
  matches!(state, JobState::Queued | JobState::Running)
}

impl Inner {
  fn lock(&self) -> MutexGuard<'_, Vec<Entry>> {
    // a panic in one worker must not take the whole queue down
    self.entries.lock().unwrap_or_else(|e| e.into_inner())
  }

  /// Applies `f` to the job and emits a snapshot when `f` reports a change. The snapshot is
  /// emitted after the lock is released, so a slow sink cannot stall other workers.
  fn modify(&self, seq: u64, f: impl FnOnce(&mut DownloadJob) -> bool) -> bool {
    let snapshot = {
      let mut entries = self.lock();
      entries.iter_mut().find(|e| e.seq == seq).and_then(|e| f(&mut e.job).then(|| e.job.clone()))
    };
    snapshot.inspect(|job| (self.on_change)(job)).is_some()
  }

  async fn work(self: Arc<Self>, seq: u64) {
    // fails only when the semaphore is closed, which never happens
    let Ok(_permit) = self.permits.clone().acquire_owned().await else {
      return;
    };
    let Some((id, quality)) = self.lock().iter().find(|e| e.seq == seq).map(|e| (e.job.id.clone(), e.quality))
    else {
      return;
    };
    // a job cancelled while it waited is no longer queued
    let started = self.modify(seq, |j| {
      let queued = j.state == JobState::Queued;
      if queued {
        j.state = JobState::Running;
      }
      queued
    });
    if !started {
      return;
    }
    log::info!("downloading {id}");

    let sink: ProgressSink = {
      let inner = self.clone();
      Arc::new(move |percent| {
        // whole percent steps are enough for a progress bar and keep the event traffic low
        inner.modify(seq, |j| {
          let changed = percent as u32 != j.percent as u32;
          if changed {
            j.percent = percent;
          }
          changed
        });
      })
    };
    match self.downloader.download(&id, quality, &self.dir, Some(sink)).await {
      Ok(file) => {
        log::info!("downloaded {id} ({} bytes)", file.size_bytes);
        self.modify(seq, |j| {
          j.state = JobState::Done;
          j.percent = 100.0;
          j.file = Some(file);
          true
        });
      }
      Err(e) => {
        log::warn!("download of {id} failed: {e}");
        self.modify(seq, |j| {
          j.state = JobState::Failed;
          j.error = Some(e.to_string());
          true
        });
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{error::{MediaError, Result}, model::DownloadedFile};
  use async_trait::async_trait;
  use std::{path::Path, time::Duration};

  /// Holds every download until the test lets one through, and records what started.
  struct Gated {
    gate: Arc<Semaphore>,
    started: Arc<Mutex<Vec<String>>>,
    fail: bool,
  }

  #[async_trait]
  impl Downloader for Gated {
    async fn download(
      &self,
      id: &str,
      _: AudioQuality,
      dir: &Path,
      _: Option<ProgressSink>,
    ) -> Result<DownloadedFile> {
      self.started.lock().unwrap().push(id.to_owned());
      self.gate.acquire().await.unwrap().forget();
      if self.fail {
        return Err(MediaError::Engine("boom".into()));
      }
      Ok(DownloadedFile { id: id.to_owned(), path: dir.join(id), size_bytes: 1 })
    }
  }

  struct Rig {
    queue: DownloadQueue,
    gate: Arc<Semaphore>,
    started: Arc<Mutex<Vec<String>>>,
    seen: Arc<Mutex<Vec<(String, JobState)>>>,
  }

  fn rig(parallel: usize, fail: bool) -> Rig {
    let gate = Arc::new(Semaphore::new(0));
    let started = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink: JobSink = {
      let seen = seen.clone();
      Arc::new(move |j| seen.lock().unwrap().push((j.id.clone(), j.state)))
    };
    let downloader = Gated { gate: gate.clone(), started: started.clone(), fail };
    let queue = DownloadQueue::new(Arc::new(downloader), "/tmp".into(), parallel, Handle::current(), sink);
    Rig { queue, gate, started, seen }
  }

  async fn until(cond: impl Fn() -> bool) {
    tokio::time::timeout(Duration::from_secs(2), async {
      while !cond() {
        tokio::task::yield_now().await;
      }
    })
    .await
    .expect("condition not reached");
  }

  fn states(q: &DownloadQueue) -> Vec<JobState> {
    q.list().iter().map(|j| j.state).collect()
  }

  #[tokio::test]
  async fn downloads_in_order_with_the_parallel_cap() {
    let r = rig(1, false);
    for id in ["a", "b", "c"] {
      r.queue.enqueue(id, AudioQuality::High);
    }
    until(|| r.started.lock().unwrap().len() == 1).await;
    assert_eq!(states(&r.queue), [JobState::Running, JobState::Queued, JobState::Queued]);

    r.gate.add_permits(1);
    until(|| r.started.lock().unwrap().len() == 2).await;
    assert_eq!(states(&r.queue), [JobState::Done, JobState::Running, JobState::Queued]);
    assert_eq!(*r.started.lock().unwrap(), ["a", "b"]);
  }

  #[tokio::test]
  async fn parallel_downloads_share_the_cap() {
    let r = rig(2, false);
    for id in ["a", "b", "c"] {
      r.queue.enqueue(id, AudioQuality::High);
    }
    until(|| r.started.lock().unwrap().len() == 2).await;
    tokio::task::yield_now().await;
    assert_eq!(r.started.lock().unwrap().len(), 2);
  }

  #[tokio::test]
  async fn enqueueing_an_active_id_returns_the_same_job() {
    let r = rig(1, false);
    r.queue.enqueue("a", AudioQuality::High);
    r.queue.enqueue("a", AudioQuality::Low);
    assert_eq!(r.queue.list().len(), 1);
  }

  #[tokio::test]
  async fn only_a_queued_job_can_be_cancelled() {
    let r = rig(1, false);
    r.queue.enqueue("a", AudioQuality::High);
    r.queue.enqueue("b", AudioQuality::High);
    until(|| r.started.lock().unwrap().len() == 1).await;
    assert!(!r.queue.cancel("a"));
    assert!(r.queue.cancel("b"));

    r.gate.add_permits(2);
    until(|| states(&r.queue)[0] == JobState::Done).await;
    tokio::task::yield_now().await;
    assert_eq!(states(&r.queue), [JobState::Done, JobState::Cancelled]);
    assert_eq!(*r.started.lock().unwrap(), ["a"]);
  }

  #[tokio::test]
  async fn a_failed_download_keeps_its_reason() {
    let r = rig(1, true);
    r.queue.enqueue("a", AudioQuality::High);
    r.gate.add_permits(1);
    until(|| states(&r.queue)[0] == JobState::Failed).await;
    assert_eq!(r.queue.list()[0].error.as_deref(), Some("boom"));
  }

  #[tokio::test]
  async fn every_state_change_is_emitted_once() {
    let r = rig(1, false);
    r.queue.enqueue("a", AudioQuality::High);
    r.gate.add_permits(1);
    until(|| states(&r.queue)[0] == JobState::Done).await;
    let seen: Vec<_> = r.seen.lock().unwrap().iter().map(|(_, s)| *s).collect();
    assert_eq!(seen, [JobState::Queued, JobState::Running, JobState::Done]);
  }

  #[tokio::test]
  async fn finished_jobs_can_be_cleared_and_requested_again() {
    let r = rig(1, false);
    r.queue.enqueue("a", AudioQuality::High);
    r.gate.add_permits(1);
    until(|| states(&r.queue)[0] == JobState::Done).await;
    r.queue.enqueue("a", AudioQuality::High);
    assert_eq!(states(&r.queue).len(), 1);
    r.gate.add_permits(1);
    until(|| states(&r.queue)[0] == JobState::Done).await;
    r.queue.clear_finished();
    assert!(r.queue.list().is_empty());
  }
}
