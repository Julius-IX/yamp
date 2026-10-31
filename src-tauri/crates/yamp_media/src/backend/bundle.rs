use super::{Downloader, MediaSource, Updater};
use std::sync::Arc;

/// The three parts the app works with. They may come from one provider or from different ones.
#[derive(Clone)]
pub struct Backend {
  pub source: Arc<dyn MediaSource>,
  pub downloader: Arc<dyn Downloader>,
  pub updater: Arc<dyn Updater>,
}

impl Backend {
  /// For a provider that implements all three parts itself.
  pub fn single<T: MediaSource + Downloader + Updater + 'static>(provider: T) -> Self {
    let provider = Arc::new(provider);
    Self { source: provider.clone(), downloader: provider.clone(), updater: provider }
  }
}
