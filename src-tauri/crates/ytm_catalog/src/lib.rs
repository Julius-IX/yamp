//! The YouTube Music implementation of [`yamp_media::Catalog`].
//!
//! It is a separate crate so that `ytmapi-rs` and everything about the shape of YouTube Music's
//! responses stays in one place. The app only knows the shapes of `yamp-media`. When the
//! provider changes, or the library is swapped for another one, only this crate is touched.
mod browse;
mod catalog;
mod convert;

pub use catalog::YtMusicCatalog;
