use serde::Serialize;
use ts_types::TsType;

#[derive(Debug, Clone, Serialize, TsType)]
#[serde(rename_all = "camelCase")]
pub struct EngineStatus {
  pub installed: bool,
  pub version: Option<String>,
}
