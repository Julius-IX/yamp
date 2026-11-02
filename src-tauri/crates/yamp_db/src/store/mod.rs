//! One store per area of the library. Each is a thin handle on the shared pool.

mod album;
mod artist;
mod like;
pub(crate) mod ordered;
pub(crate) mod track;

pub use album::{Album, Albums, SavedAlbum};
pub use artist::{Artist, Artists, FollowedArtist};
pub use like::Likes;
pub use track::{Track, Tracks};
