use serde::Serialize;
use std::path::PathBuf;
use ts_types::TsType;

#[derive(Debug, Clone, Serialize, TsType)]
#[serde(rename_all = "camelCase")]
pub struct DownloadedFile {
  pub id: String,
  pub path: PathBuf,
  pub size_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TsType)]
#[serde(rename_all = "lowercase")]
pub enum JobState {
  Queued,
  Running,
  Done,
  Failed,
  Cancelled,
}

/// A download as the queue sees it. Every change is emitted as a full snapshot, so a consumer
/// only has to replace the job with the same `id`.
#[derive(Debug, Clone, Serialize, TsType)]
#[serde(rename_all = "camelCase")]
pub struct DownloadJob {
  pub id: String,
  pub state: JobState,
  pub percent: f32,
  pub file: Option<DownloadedFile>,
  pub error: Option<String>,
}
