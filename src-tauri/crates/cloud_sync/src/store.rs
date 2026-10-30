use std::{path::PathBuf, sync::Arc};

use crate::{Error, Result};

/// Largest object a backend accepts. Every service takes a single request upload at
/// this size, which avoids resumable upload sessions. Sync segments stay far below it.
pub const MAX_OBJECT_SIZE: usize = 4 * 1024 * 1024;

pub(crate) fn check_size(len: usize) -> Result<()> {
  if len > MAX_OBJECT_SIZE {
    return Err(Error::TooLarge {
      size: len,
      max: MAX_OBJECT_SIZE,
    });
  }
  Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectMeta {
  pub key: String,
  pub size: u64,
  /// Milliseconds since the Unix epoch, absent when the backend does not say.
  pub modified_ms: Option<i64>,
  /// Opaque value that changes with the content. It is only meaningful for
  /// equality checks against an earlier value from the same backend.
  pub revision: String,
}

/// A flat key to bytes store inside a private folder of the user's cloud account.
///
/// There are no conditional writes on purpose. Google Drive offers none, so callers
/// keep a single writer per key, for example by putting a device id into the key.
/// Every method blocks, so async callers run them on a blocking thread.
pub trait CloudStore: Send + Sync {
  /// Objects whose key starts with `prefix`, sorted by key.
  fn list(&self, prefix: &str) -> Result<Vec<ObjectMeta>>;

  fn stat(&self, key: &str) -> Result<Option<ObjectMeta>>;

  fn get(&self, key: &str) -> Result<Vec<u8>>;

  /// Creates the object or replaces its content.
  fn put(&self, key: &str, data: &[u8]) -> Result<ObjectMeta>;

  /// Removing a missing object is not an error, so a retried delete stays safe.
  fn delete(&self, key: &str) -> Result<()>;
}

/// Supplies OAuth access tokens. The crate does not run any login flow, so the
/// consent screen, token storage and refreshing stay with the application.
///
/// Scopes that keep the store inside an app private folder:
/// Google `drive.appdata`, Microsoft `Files.ReadWrite.AppFolder`.
pub trait TokenProvider: Send + Sync {
  /// Called with `force_refresh` set after the service rejected the previous token.
  fn token(&self, force_refresh: bool) -> Result<String>;
}

/// Inputs for [`open`](crate::open). Each backend reads only what it needs.
#[derive(Clone, Default)]
pub struct Settings {
  /// Needed by the Google Drive and OneDrive backends.
  pub tokens: Option<Arc<dyn TokenProvider>>,
  /// Needed by the iCloud backend, the app's ubiquity container directory.
  pub container: Option<PathBuf>,
}

impl Settings {
  pub fn with_tokens(mut self, tokens: Arc<dyn TokenProvider>) -> Self {
    self.tokens = Some(tokens);
    self
  }

  pub fn with_container(mut self, dir: impl Into<PathBuf>) -> Self {
    self.container = Some(dir.into());
    self
  }
}
