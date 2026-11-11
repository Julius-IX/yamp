//! Moving a library between Yamp and other music services through CSV files.
//!
//! The crate has no dependencies on purpose, so `cargo test` here builds nothing else. It only
//! deals with text. Searching the provider and writing to the library is for the app, which
//! asks the crate what to look for and which result is the right one:
//!
//! - [`parse`] turns the file of Spotify, SoundCloud or Yamp itself into [`Entry`] values.
//! - [`query`] and [`pick`] find the Yamp track or album that an entry stands for.
//!
//! Services word their columns differently and none of them promises a format. Columns are
//! therefore found by name, with several names for the same thing, instead of by position.

mod entry;
mod matching;
mod read;
mod text;

pub use entry::{Entry, Failure, Kind, Reason, Source};
pub use matching::{Candidate, pick, query};
pub use read::{ReadError, Parsed, parse};
