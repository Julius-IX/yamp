use serde::{Deserialize, Serialize};

use std::time::Instant;

use crate::{gdrive, onedrive, CloudStore, Error, ObjectMeta, Result, Settings};

/// A cloud service that can hold the store.
///
/// Devices only see each other's data when they use the same account of the same
/// service, so the choice is made at runtime by the user and not at build time.
/// The names are stable and meant to be persisted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Provider {
  #[serde(rename = "google-drive")]
  GoogleDrive,
  #[serde(rename = "onedrive")]
  OneDrive,
  /// Only exists on Apple targets, see [`Provider::cross_platform`].
  #[cfg(target_vendor = "apple")]
  #[serde(rename = "icloud")]
  ICloud,
}

#[cfg(target_vendor = "apple")]
const AVAILABLE: &[Provider] = &[Provider::GoogleDrive, Provider::OneDrive, Provider::ICloud];
#[cfg(not(target_vendor = "apple"))]
const AVAILABLE: &[Provider] = &[Provider::GoogleDrive, Provider::OneDrive];

impl Provider {
  /// Providers this build can talk to.
  pub fn available() -> &'static [Provider] {
    AVAILABLE
  }

  /// Whether devices of every platform can use the provider. iCloud has no API
  /// outside Apple platforms, so it only connects Apple devices with each other.
  /// Syncing Android with iOS or Linux with macOS needs Google Drive or OneDrive.
  pub fn cross_platform(self) -> bool {
    match self {
      Provider::GoogleDrive | Provider::OneDrive => true,
      #[cfg(target_vendor = "apple")]
      Provider::ICloud => false,
    }
  }
}

/// Opens the store of a provider. Each provider reads only the settings it needs.
pub fn open(provider: Provider, settings: Settings) -> Result<Box<dyn CloudStore>> {
  log::info!("opening {provider:?} store");
  let store = open_backend(provider, settings)?;
  Ok(Box::new(Logged { inner: store }))
}

fn open_backend(provider: Provider, settings: Settings) -> Result<Box<dyn CloudStore>> {
  match provider {
    Provider::GoogleDrive => gdrive::open(settings),
    Provider::OneDrive => onedrive::open(settings),
    #[cfg(target_vendor = "apple")]
    Provider::ICloud => crate::icloud::open(settings),
  }
}

/// Logs every operation at debug level and failures at warn level, in one place
/// instead of in each backend. A missing object is an expected answer, not a failure.
struct Logged {
  inner: Box<dyn CloudStore>,
}

fn logged<T>(op: &str, target: &str, started: Instant, res: Result<T>) -> Result<T> {
  let ms = started.elapsed().as_millis();
  match &res {
    Ok(_) => log::debug!("{op} {target} ok in {ms}ms"),
    Err(Error::NotFound(_)) => log::debug!("{op} {target} not found in {ms}ms"),
    Err(e) => log::warn!("{op} {target} failed after {ms}ms: {e}"),
  }
  res
}

impl CloudStore for Logged {
  fn list(&self, prefix: &str) -> Result<Vec<ObjectMeta>> {
    let t = Instant::now();
    logged("list", prefix, t, self.inner.list(prefix))
  }

  fn stat(&self, key: &str) -> Result<Option<ObjectMeta>> {
    let t = Instant::now();
    logged("stat", key, t, self.inner.stat(key))
  }

  fn get(&self, key: &str) -> Result<Vec<u8>> {
    let t = Instant::now();
    logged("get", key, t, self.inner.get(key))
  }

  fn put(&self, key: &str, data: &[u8]) -> Result<ObjectMeta> {
    let t = Instant::now();
    logged(
      "put",
      &format!("{key} ({} bytes)", data.len()),
      t,
      self.inner.put(key, data),
    )
  }

  fn delete(&self, key: &str) -> Result<()> {
    let t = Instant::now();
    logged("delete", key, t, self.inner.delete(key))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn names_are_stable() {
    assert_eq!(
      serde_json::to_string(&Provider::GoogleDrive).unwrap(),
      "\"google-drive\""
    );
    let p: Provider = serde_json::from_str("\"onedrive\"").unwrap();
    assert_eq!(p, Provider::OneDrive);
  }

  #[test]
  fn rest_providers_are_always_available() {
    assert!(Provider::available().contains(&Provider::GoogleDrive));
    assert!(Provider::available().contains(&Provider::OneDrive));
    assert!(Provider::available()
      .iter()
      .filter(|p| !p.cross_platform())
      .all(|_| cfg!(target_vendor = "apple")));
  }

  #[test]
  fn missing_settings_are_reported() {
    let err = open(Provider::GoogleDrive, Settings::default())
      .err()
      .unwrap();
    assert!(matches!(err, crate::Error::MissingSetting("tokens")));
  }
}
