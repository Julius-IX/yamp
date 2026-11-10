//! Discord Rich Presence, the "listening to" card on the Discord profile of the user. It has no
//! Tauri dependency, so `cargo test` here builds only this crate.
//!
//! Only a desktop can talk to Discord, over a local socket. On a phone every call does nothing
//! and `available` is false. The Discord application that owns the card is made in the developer
//! portal. Its id is baked in when the crate is compiled, see `build.rs`, and a build without it
//! is not `available`, which is how the settings know to hide the option.
use std::{
  fmt,
  sync::{Arc, Mutex},
};

const APP_ID: Option<&str> = option_env!("YAMP_DISCORD_APP_ID");

fn app_id() -> Option<&'static str> {
  APP_ID.map(str::trim).filter(|id| !id.is_empty())
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
type Client = discord_rich_presence::DiscordIpcClient;
#[cfg(any(target_os = "android", target_os = "ios"))]
type Client = ();

/// Whether this build can show a presence, which needs a desktop and a Discord application id.
pub fn available() -> bool {
  cfg!(not(any(target_os = "android", target_os = "ios"))) && app_id().is_some()
}

#[derive(Debug)]
pub enum Error {
  /// The build has no application id.
  NotSetUp,
  /// Discord is not running or refused the update.
  Ipc(String),
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::NotSetUp => f.write_str("discord presence is not set up in this build"),
      Self::Ipc(detail) => f.write_str(detail),
    }
  }
}

impl std::error::Error for Error {}

/// What the card shows. The times are in milliseconds since the Unix epoch.
pub struct Track<'a> {
  pub title: &'a str,
  pub artist: &'a str,
  pub cover: Option<&'a str>,
  pub start_ms: i64,
  /// Set when the length is known, which makes Discord draw a progress bar. Without it the card
  /// only counts up.
  pub end_ms: Option<i64>,
}

/// The connection to Discord, opened by the first `show`. It is dropped when a send fails, which
/// is what happens when Discord was closed, so the next `show` connects again.
#[derive(Clone, Default)]
pub struct Presence(Arc<Mutex<Option<Client>>>);

/// Discord refuses a line of less than 2 characters and of more than 128.
#[cfg_attr(any(target_os = "android", target_os = "ios"), allow(dead_code))]
fn field(text: &str) -> String {
  let cut: String = text.chars().take(128).collect();
  if cut.chars().count() < 2 { format!("{cut}\u{200B}") } else { cut }
}

/// Discord only loads an image from a web address, and a cached local cover has none.
#[cfg_attr(any(target_os = "android", target_os = "ios"), allow(dead_code))]
fn cover_url(cover: Option<&str>) -> Option<&str> {
  cover.filter(|u| u.starts_with("http") && u.len() <= 256)
}

impl Presence {
  #[cfg(not(any(target_os = "android", target_os = "ios")))]
  pub fn show(&self, track: &Track) -> Result<(), Error> {
    use discord_rich_presence::{
      DiscordIpc, DiscordIpcClient,
      activity::{Activity, ActivityType, Assets, StatusDisplayType, Timestamps},
    };

    let id = app_id().ok_or(Error::NotSetUp)?;
    let stamps = match track.end_ms {
      Some(end) => Timestamps::new().start(track.start_ms).end(end),
      None => Timestamps::new().start(track.start_ms),
    };
    let (details, state) = (field(track.title), field(track.artist));
    let mut activity = Activity::new()
      .activity_type(ActivityType::Listening)
      .status_display_type(StatusDisplayType::State)
      .details(&details)
      .state(&state)
      .timestamps(stamps);
    if let Some(url) = cover_url(track.cover) {
      activity = activity.assets(Assets::new().large_image(url));
    }

    let ipc = |e: discord_rich_presence::error::Error| Error::Ipc(e.to_string());
    let mut slot = self.0.lock().unwrap_or_else(|e| e.into_inner());
    // a send that fails on an old connection is tried once more on a new one
    for attempt in 0..2 {
      if slot.is_none() {
        let mut client = DiscordIpcClient::new(id);
        client.connect().map_err(ipc)?;
        *slot = Some(client);
      }
      let sent = slot.as_mut().map(|client| client.set_activity(activity.clone()));
      match sent {
        Some(Ok(())) => return Ok(()),
        Some(Err(e)) if attempt == 1 => {
          *slot = None;
          return Err(ipc(e));
        }
        _ => *slot = None,
      }
    }
    Ok(())
  }

  #[cfg(any(target_os = "android", target_os = "ios"))]
  pub fn show(&self, _: &Track) -> Result<(), Error> {
    Ok(())
  }

  /// Removes the card. Nothing happens when there is no connection.
  #[cfg(not(any(target_os = "android", target_os = "ios")))]
  pub fn hide(&self) {
    use discord_rich_presence::DiscordIpc;

    let mut slot = self.0.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(client) = slot.as_mut() {
      if client.clear_activity().is_err() {
        *slot = None;
      }
    }
  }

  #[cfg(any(target_os = "android", target_os = "ios"))]
  pub fn hide(&self) {}
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn short_lines_are_padded() {
    assert_eq!(field("a"), "a\u{200B}");
    assert_eq!(field(""), "\u{200B}");
    assert_eq!(field("ab"), "ab");
  }

  #[test]
  fn long_lines_are_cut_by_characters() {
    let long = "é".repeat(200);
    assert_eq!(field(&long).chars().count(), 128);
  }

  #[test]
  fn only_web_covers_are_used() {
    assert_eq!(cover_url(Some("https://i.example/a.jpg")), Some("https://i.example/a.jpg"));
    assert_eq!(cover_url(Some("/home/user/cover.jpg")), None);
    assert_eq!(cover_url(Some(&format!("https://{}", "a".repeat(300)))), None);
    assert_eq!(cover_url(None), None);
  }
}
