use crate::{error::Result, model::EngineStatus};
use async_trait::async_trait;

/// Provider tooling is the part that breaks first when a site changes, so refreshing it
/// is part of the contract.
#[async_trait]
pub trait Updater: Send + Sync {
  async fn status(&self) -> Result<EngineStatus>;
  /// Installs the tooling when missing, otherwise moves it to the latest release.
  async fn update(&self) -> Result<EngineStatus>;
}
