use serde::{Deserialize, Serialize};
use ts_types::TsType;

/// A directly playable audio URL. Such URLs are short lived and often bound to the
/// requesting IP, so `expires_at` tells when to resolve a fresh one.
#[derive(Debug, Clone, Serialize, TsType)]
#[serde(rename_all = "camelCase")]
pub struct StreamInfo {
  pub url: String,
  pub ext: String,
  pub mime: String,
  pub bitrate_kbps: Option<f64>,
  /// Unix seconds. Absent when the provider does not say.
  pub expires_at: Option<u64>,
}

/// Coarse on purpose. Each backend maps a level to whatever its formats allow.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TsType)]
#[serde(rename_all = "lowercase")]
pub enum AudioQuality {
  Low,
  Medium,
  #[default]
  High,
}
