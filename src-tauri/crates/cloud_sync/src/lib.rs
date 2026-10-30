//! File storage on the user's own cloud account, meant as the transport for syncing
//! data between devices.
//!
//! The crate only moves bytes. It knows nothing about what is stored, which keeps it
//! reusable and testable without the application around it.
//!
//! The service is chosen at runtime through [`Provider`], because devices only sync
//! when they share an account of the same service. The REST backends (Google Drive,
//! OneDrive) are compiled on every platform so that any mix of devices can use them.
//! iCloud has no API outside Apple platforms, so it exists on Apple targets only.
//! [`open`] returns the chosen backend behind the [`CloudStore`] trait.

mod error;
mod gdrive;
mod http;
mod key;
mod onedrive;
mod provider;
mod store;

pub mod fs;

#[cfg(target_vendor = "apple")]
mod icloud;

pub use error::{Error, Result};
pub use key::{validate_key, validate_prefix};
pub use provider::{open, Provider};
pub use store::{CloudStore, ObjectMeta, Settings, TokenProvider, MAX_OBJECT_SIZE};
