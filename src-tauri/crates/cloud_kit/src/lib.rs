//! Building blocks on top of `cloud-sync` for sharing data between devices.
//!
//! The crate has no schemas. It offers shapes to build them from:
//!
//! - [`Dir`] is a directory with files and subdirectories, plus JSON documents with
//!   create, read, update and delete.
//! - [`Log`] is an append only sequence of entries ordered by time, with a cursor to
//!   resume from. This is the shape for shipping changes.
//! - [`Blobs`] are big objects such as snapshots, chunked and checksummed.
//!
//! Everything works on any [`cloud_sync::CloudStore`]. Backends have no atomic
//! create or compare and swap, so the rule throughout is a single writer per name.
//! One directory per device gives that for free.

mod blob;
mod dir;
mod error;
mod json;
mod log;
pub mod stamp;

pub use blob::{BlobInfo, Blobs};
pub use dir::Dir;
pub use error::{Error, Result};
pub use log::{entry_time_ms, Log};

#[cfg(test)]
mod tests;
