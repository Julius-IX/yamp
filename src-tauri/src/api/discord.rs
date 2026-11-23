//! Commands for the Discord presence. The connection to Discord is in `yamp-discord`, this only
//! gives the frontend access to it. Android has no local socket to Discord, so the card goes to
//! the Social SDK of the session plugin there, and iOS does nothing.
use super::err;
use tauri::State;
use yamp_discord::Track;
use yamp_session::Session;

pub use yamp_discord::Presence;

/// Whether this build can show a presence, which needs a Discord application id. Android also
/// needs the Social SDK, which is not part of every build, so its plugin is asked.
#[tauri_groups::command(discord)]
pub async fn available(session: State<'_, Session>) -> Result<bool, String> {
  #[cfg(target_os = "android")]
  {
    if yamp_discord::app_id().is_none() {
      return Ok(false);
    }
    let found = session.native("discord", serde_json::json!({ "op": "available" })).await.map_err(err)?;
    Ok(found.get("available").and_then(serde_json::Value::as_bool).unwrap_or(false))
  }
  #[cfg(not(target_os = "android"))]
  {
    let _ = session;
    Ok(yamp_discord::available())
  }
}

/// The card as the plugin reads it. The names are the ones of `DiscordHost` in Kotlin.
#[cfg(target_os = "android")]
fn payload(app_id: &str, track: &Track) -> serde_json::Value {
  let card = yamp_discord::Card::new(track);
  serde_json::json!({
    "op": "update",
    "appId": app_id,
    "card": {
      "name": card.name,
      "details": card.details,
      "state": card.state,
      "detailsUrl": card.details_url,
      "stateUrl": card.state_url,
      "cover": card.cover,
      "hover": card.hover,
      "badgeUrl": card.badge_url,
      "siteUrl": card.site_url,
      "buttons": card.buttons,
      "startMs": card.start_ms,
      "endMs": card.end_ms,
    },
  })
}

/// Shows the track as the activity. The times are in milliseconds since the Unix epoch, the track
/// is shown as started at `start_ms` and as ending at `end_ms` when its length is known. `url` and
/// `artist_url` are web pages that the card links to, they are left out for what has none.
#[tauri_groups::command(discord)]
pub async fn update(
  presence: State<'_, Presence>,
  session: State<'_, Session>,
  title: String,
  artist: String,
  cover: Option<String>,
  album: Option<String>,
  start_ms: i64,
  end_ms: Option<i64>,
  url: Option<String>,
  artist_url: Option<String>,
) -> Result<(), String> {
  #[cfg(target_os = "android")]
  {
    let _ = presence;
    let Some(app_id) = yamp_discord::app_id() else { return Ok(()) };
    let track = Track {
      title: &title,
      artist: &artist,
      cover: cover.as_deref(),
      album: album.as_deref(),
      start_ms,
      end_ms,
      url: url.as_deref(),
      artist_url: artist_url.as_deref(),
    };
    session.native("discord", payload(app_id, &track)).await.map(|_| ()).map_err(err)
  }
  #[cfg(not(target_os = "android"))]
  {
    let _ = session;
    let presence = presence.inner().clone();
    // the socket can block for a moment, and a command on the main thread would hold up the window
    tauri::async_runtime::spawn_blocking(move || {
      presence.show(&Track {
        title: &title,
        artist: &artist,
        cover: cover.as_deref(),
        album: album.as_deref(),
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
}

#[tauri_groups::command(discord)]
pub async fn clear(presence: State<'_, Presence>, session: State<'_, Session>) -> Result<(), String> {
  #[cfg(target_os = "android")]
  {
    let _ = presence;
    session.native("discord", serde_json::json!({ "op": "clear" })).await.map(|_| ()).map_err(err)
  }
  #[cfg(not(target_os = "android"))]
  {
    let _ = session;
    let presence = presence.inner().clone();
    tauri::async_runtime::spawn_blocking(move || presence.hide()).await.map_err(err)
  }
}
