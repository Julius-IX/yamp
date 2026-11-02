//! One store per area of the library. Each is a thin handle on the shared pool.

mod album;
mod artist;
mod download;
mod like;
pub(crate) mod ordered;
mod playlist;
pub(crate) mod track;

pub use album::{Album, Albums, SavedAlbum};
pub use artist::{Artist, Artists, FollowedArtist};
pub use download::{Download, Downloads};
pub use like::Likes;
pub use playlist::{Playlist, Playlists};
pub use track::{Track, Tracks};

pub(crate) use download::check_name as download_name_check;
