//! Commands for the Discord presence. The connection to Discord is in `yamp-discord`, this only
//! gives the frontend access to it. On a phone the commands do nothing.
use super::err;
use tauri::State;
use yamp_discord::Track;

pub use yamp_discord::Presence;

/// Whether this build can show a presence, which needs a desktop and a Discord application id.
#[tauri_groups::command(discord)]
pub fn available() -> bool {
  yamp_discord::available()
}

/// Shows the track as the activity. The times are in milliseconds since the Unix epoch, the track
/// is shown as started at `start_ms` and as ending at `end_ms` when its length is known. `url` and
/// `artist_url` are web pages that the card links to, they are left out for what has none.
#[tauri_groups::command(discord)]
pub async fn update(
  presence: State<'_, Presence>,
  title: String,
  artist: String,
  cover: Option<String>,
  start_ms: i64,
  end_ms: Option<i64>,
  url: Option<String>,
  artist_url: Option<String>,
) -> Result<(), String> {
  let presence = presence.inner().clone();
  // the socket can block for a moment, and a command on the main thread would hold up the window
  tauri::async_runtime::spawn_blocking(move || {
    presence.show(&Track {
      title: &title,
      artist: &artist,
      cover: cover.as_deref(),
      start_ms,
      end_ms,
      url: url.as_deref(),
      artist_url: artist_url.as_deref(),
    })
  })
  .await
  .map_err(err)?
  .map_err(err)
}

#[tauri_groups::command(discord)]
pub async fn clear(presence: State<'_, Presence>) -> Result<(), String> {
  let presence = presence.inner().clone();
  tauri::async_runtime::spawn_blocking(move || presence.hide()).await.map_err(err)
}
