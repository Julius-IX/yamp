//! iCloud has no HTTP API for third party apps. Files sync through the app's
//! ubiquity container, a directory the OS keeps in sync, so this backend is a
//! [`LocalDirStore`] on that directory.
//!
//! Resolving the container needs a native call (`URLForUbiquityContainerIdentifier`)
//! and the iCloud entitlement, which is why the path is passed in through settings.
//! The data only reaches other Apple devices of the same Apple ID.

use crate::{fs::LocalDirStore, CloudStore, Error, Result, Settings};

pub(crate) fn open(settings: Settings) -> Result<Box<dyn CloudStore>> {
  let dir = settings
    .container
    .ok_or(Error::MissingSetting("container"))?;
  Ok(Box::new(LocalDirStore::new(dir)?))
}
