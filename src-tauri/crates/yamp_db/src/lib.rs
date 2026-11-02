//! SQLite storage for the music library.
//!
//! The crate owns the schema and every query on it. The app talks to the typed stores
//! handed out by [`Db`] and never writes SQL itself.
//!
//! Queries are checked at compile time by the `sqlx` macros and the schema comes from the
//! migrations at the workspace root, see `migrate`. The crate has no Tauri dependency on
//! purpose, so it can be checked and tested alone like `yamp-media`.

mod clock;
mod db;
mod error;
mod link;
mod media;
mod migrate;
mod store;

pub use db::Db;
pub use error::{Error, Result};
pub use media::{MediaStore, RepairReport};
pub use store::*;
