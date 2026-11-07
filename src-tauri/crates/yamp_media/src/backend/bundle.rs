use super::{Catalog, Downloader, MediaSource, NoCatalog, Updater};
use std::sync::Arc;

/// The four parts the app works with. They may come from one provider or from different ones.
#[derive(Clone)]
pub struct Backend {
  pub source: Arc<dyn MediaSource>,
  pub downloader: Arc<dyn Downloader>,
  pub updater: Arc<dyn Updater>,
  pub catalog: Arc<dyn Catalog>,
}

impl Backend {
  /// For a provider that implements source, downloader and updater itself. It has no catalog
  /// until one is added with [`Backend::with_catalog`].
  pub fn single<T: MediaSource + Downloader + Updater + 'static>(provider: T) -> Self {
    let provider = Arc::new(provider);
    Self {
      source: provider.clone(),
      downloader: provider.clone(),
      updater: provider,
      catalog: Arc::new(NoCatalog),
    }
  }

  pub fn with_catalog(mut self, catalog: Arc<dyn Catalog>) -> Self {
    self.catalog = catalog;
    self
  }
}
