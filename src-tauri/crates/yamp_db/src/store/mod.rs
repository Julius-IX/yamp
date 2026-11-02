//! One store per area of the library. Each is a thin handle on the shared pool.

mod like;
pub(crate) mod track;

pub use like::Likes;
pub use track::{Track, Tracks};
