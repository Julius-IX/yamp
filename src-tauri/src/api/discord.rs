//! Discord Rich Presence, the "listening to" card on the Discord profile of the user. Only a
//! desktop can talk to Discord, over a local socket, so on a phone the commands do nothing.
//!
//! The Discord application that owns the card is made in the Discord developer portal, its id is
//! given to the build as `YAMP_DISCORD_APP_ID`. A build without it reports `available` as false
//! and the settings hide the option.
use std::sync::{Arc, Mutex};

use super::err;
use tauri::State;

const APP_ID: Option<&str> = option_env!("YAMP_DISCORD_APP_ID");

fn app_id() -> Option<&'static str> {
  APP_ID.map(str::trim).filter(|id| !id.is_empty())
}

#[cfg(desktop)]
type Client = discord_rich_presence::DiscordIpcClient;
#[cfg(not(desktop))]
type Client = ();

/// The connection to Discord, opened by the first update. It is dropped when a send fails, which
/// is what happens when Discord was closed, so the next update connects again.
#[derive(Clone, Default)]
pub struct Presence(Arc<Mutex<Option<Client>>>);

/// Discord refuses a line of less than 2 characters and of more than 128.
#[cfg(desktop)]
fn field(text: &str) -> String {
  let cut: String = text.chars().take(128).collect();
  if cut.chars().count() < 2 { format!("{cut}\u{200B}") } else { cut }
}

impl Presence {
  #[cfg(desktop)]
  fn show(&self, title: &str, artist: &str, cover: Option<&str>, start_ms: i64, end_ms: Option<i64>) -> Result<(), String> {
    use discord_rich_presence::{
      DiscordIpc, DiscordIpcClient,
      activity::{Activity, ActivityType, Assets, StatusDisplayType, Timestamps},
    };

    let id = app_id().ok_or("discord presence is not set up in this build")?;
    // both times make Discord draw a progress bar, one alone only counts up
    let stamps = match end_ms {
      Some(end) => Timestamps::new().start(start_ms).end(end),
      None => Timestamps::new().start(start_ms),
    };
    let (details, state) = (field(title), field(artist));
    let mut activity = Activity::new()
      .activity_type(ActivityType::Listening)
      .status_display_type(StatusDisplayType::State)
      .details(&details)
      .state(&state)
      .timestamps(stamps);
    // Discord only loads an image from a web address, a cached local cover has none
    if let Some(url) = cover.filter(|u| u.starts_with("http") && u.len() <= 256) {
      activity = activity.assets(Assets::new().large_image(url));
    }

    let mut slot = self.0.lock().unwrap_or_else(|e| e.into_inner());
    // a send that fails on an old connection is tried once more on a new one
    for attempt in 0..2 {
      if slot.is_none() {
        let mut client = DiscordIpcClient::new(id);
        client.connect().map_err(err)?;
        *slot = Some(client);
      }
      let sent = slot.as_mut().map(|client| client.set_activity(activity.clone()));
      match sent {
        Some(Ok(())) => return Ok(()),
        Some(Err(e)) if attempt == 1 => {
          *slot = None;
          return Err(err(e));
        }
        _ => *slot = None,
      }
    }
    Ok(())
  }

  #[cfg(not(desktop))]
  fn show(&self, _: &str, _: &str, _: Option<&str>, _: i64, _: Option<i64>) -> Result<(), String> {
    Ok(())
  }

  #[cfg(desktop)]
  fn hide(&self) {
    use discord_rich_presence::DiscordIpc;

    let mut slot = self.0.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(client) = slot.as_mut() {
      if client.clear_activity().is_err() {
        *slot = None;
      }
    }
  }

  #[cfg(not(desktop))]
  fn hide(&self) {}
}

/// Whether this build can show a presence, which needs a desktop and a Discord application id.
#[tauri_groups::command(discord)]
pub fn available() -> bool {
  cfg!(desktop) && app_id().is_some()
}

/// Shows the track as the activity. The times are in milliseconds since the Unix epoch, the track
/// is shown as started at `start_ms` and as ending at `end_ms` when its length is known.
#[tauri_groups::command(discord)]
pub async fn update(
  presence: State<'_, Presence>,
  title: String,
  artist: String,
  cover: Option<String>,
  start_ms: i64,
  end_ms: Option<i64>,
) -> Result<(), String> {
  let presence = presence.inner().clone();
  // the socket can block for a moment, and a command on the main thread would hold up the window
  tauri::async_runtime::spawn_blocking(move || presence.show(&title, &artist, cover.as_deref(), start_ms, end_ms))
    .await
    .map_err(err)?
}

#[tauri_groups::command(discord)]
pub async fn clear(presence: State<'_, Presence>) -> Result<(), String> {
  let presence = presence.inner().clone();
  tauri::async_runtime::spawn_blocking(move || presence.hide()).await.map_err(err)
}
