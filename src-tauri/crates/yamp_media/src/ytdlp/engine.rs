use crate::{error::Result, ProgressSink};
use async_trait::async_trait;
use serde::Serialize;

/// One `--option value` pair. An empty value makes a bare flag.
#[derive(Debug, Clone, Serialize)]
pub struct Opt {
  pub k: String,
  pub v: String,
}

/// Arguments of one yt-dlp call. The target is kept apart from the options so an engine can
/// put it after a `--` and it is never read as a flag.
#[derive(Debug, Clone, Serialize)]
pub struct YtDlpArgs {
  pub target: String,
  pub opts: Vec<Opt>,
}

impl YtDlpArgs {
  pub fn new(target: impl Into<String>) -> Self {
    Self { target: target.into(), opts: Vec::new() }
  }

  pub fn flag(mut self, k: &str) -> Self {
    self.opts.push(Opt { k: k.into(), v: String::new() });
    self
  }

  pub fn opt(mut self, k: &str, v: impl Into<String>) -> Self {
    self.opts.push(Opt { k: k.into(), v: v.into() });
    self
  }
}

/// How yt-dlp is executed on a platform. This is the only part that differs between
/// desktop (a downloaded binary) and Android (an embedded Python build behind a plugin).
#[async_trait]
pub trait YtDlpEngine: Send + Sync {
  /// Runs yt-dlp and returns its stdout. When `progress` is set, the engine reports download
  /// percentages through it while the call runs.
  async fn run(&self, args: &YtDlpArgs, progress: Option<ProgressSink>) -> Result<String>;
  /// `None` when yt-dlp is not installed.
  async fn version(&self) -> Result<Option<String>>;
  /// Installs or updates yt-dlp and returns the resulting version.
  async fn update(&self) -> Result<String>;
}
