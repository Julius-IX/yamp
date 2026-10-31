//! A small key-value store for UI state (theme, toggles, sizes), saved as one JSON file in the
//! app data directory.
//!
//! The app passes the list of keys it has when the store is opened. Keys outside that list are
//! never kept, so the file can't collect values that nothing reads. Defaults are not stored. A
//! setting that was never changed is absent and the reader supplies the default, so a changed
//! default also reaches users who never touched the setting.
//!
//! - [`Settings`] is the store and works without Tauri.
//! - `init` (feature `tauri`) opens it as managed state and registers the `settings` commands.
//! - [`peek`] and [`peek_early`] read one key before the app exists.
//!
//! Values are plain JSON on purpose. A new setting needs no migration, and the reader checks the
//! type of what it gets.

mod early;
mod store;

#[cfg(feature = "tauri")]
mod app;

pub use early::peek;
#[cfg(target_os = "linux")]
pub use early::peek_early;
pub use store::{FILE, Settings};

#[cfg(feature = "tauri")]
pub use app::init;
