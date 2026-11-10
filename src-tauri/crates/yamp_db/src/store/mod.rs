//! One store per area of the library. Each is a thin handle on the shared pool.

mod artist;
pub(crate) mod cover;
mod download;
mod like;
mod local;
pub(crate) mod ordered;
mod playlist;
pub(crate) mod track;
mod value;

pub use artist::{Artist, Artists, FollowedArtist};
pub use cover::{COVER_SIZE, PlaylistCovers};
pub use download::{Download, Downloads};
pub use like::Likes;
pub use local::{
  COVER_PREFIX, LOCAL_PREFIX, LocalUsage, Locals, ScanReport, SubDir, Totals, is_local_id, list_subdirs, track_id_for,
};
pub use playlist::{Playlist, Playlists};
pub use track::{Track, Tracks};
pub use value::Values;

pub(crate) use download::check_name as download_name_check;
