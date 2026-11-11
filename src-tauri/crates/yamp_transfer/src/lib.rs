//! Moving a library between Yamp and other music services through CSV files and the data
//! export of Spotify.
//!
//! The crate only deals with text and depends on nothing but `serde_json`, so `cargo test` here
//! builds almost nothing else. Searching the provider and writing to the library is for the app,
//! which asks the crate what to look for and which result is the right one:
//!
//! - [`parse`] turns a CSV file, or the JSON of the account data of Spotify, into [`Entry`] values.
//! - [`query`] and [`pick`] find the Yamp track or album that an entry stands for.
//! - [`library_csv`] and [`failures_csv`] write the files that leave the app.
//!
//! Services word their columns differently and none of them promises a format. Columns are
//! therefore found by name, with several names for the same thing, instead of by position.

mod entry;
mod json;
mod matching;
mod read;
mod text;
mod write;

pub use entry::{Entry, Failure, Kind, Reason, Source};
pub use matching::{Candidate, pick, query};
pub use read::{ReadError, Parsed, parse};
pub use write::{failures_csv, library_csv};
