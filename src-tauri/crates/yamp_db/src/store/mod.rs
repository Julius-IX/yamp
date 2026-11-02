//! One store per area of the library. Each is a thin handle on the shared pool.

pub(crate) mod track;

pub use track::{Track, Tracks};
