//! Command groups. Thin wrappers that give the frontend typed access to `YtApi`, which owns all
//! provider handling, and to the library in `yamp-db`. Nothing here knows which backend runs.
mod artists;
mod downloads;
mod engine;
mod likes;
mod media;
mod playlists;
mod queue;
mod session;
mod sync;
mod tracks;
mod wire;

/// Commands report failures as plain strings, the detail is already logged by `yt_api`.
fn err(e: impl ToString) -> String {
  e.to_string()
}

/// Library failures are logged here, since unlike the provider calls nothing logged them yet.
fn fail(e: yamp_db::Error) -> String {
  log::warn!("library: {e}");
  e.to_string()
}
