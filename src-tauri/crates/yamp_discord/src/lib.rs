//! Discord Rich Presence, the "listening to" card on the Discord profile of the user. It has no
//! Tauri dependency, so `cargo test` here builds only this crate.
//!
//! The card is set up like the one of a music service. The status reads "Listening to" and the
//! artist, since the person listens on yamp to something and not to yamp itself, and the card
//! names yamp where Discord allows it, as a badge on the cover and in a button. The title,
//! the artist and the cover link to the track when it has a web page. The badge links to the
//! site of yamp.
//!
//! A desktop talks to Discord over a local socket, which this crate does itself. A phone has no
//! such socket, so on Android the app hands a [`Card`] to the Discord Social SDK, which talks to the
//! Discord app of the phone. That part is in `yamp-session`, since it needs the Android plugin.
//! The text and the links of the card are built here for both, so they cannot drift apart. iOS
//! has no way to show a presence without a sign in, and every call does nothing there.
//!
//! The Discord application that owns the card is made in the developer portal. Its id is baked in
//! when the crate is compiled, see `build.rs`, and a build without it has no presence, which is
//! how the settings know to hide the option.
use std::{
  fmt,
  sync::{Arc, Mutex},
};

/// Where the button that names the app leads.
const APP_URL: &str = "https://github.com/Julius-IX/yamp";

/// Where a click on the badge leads. The button already links the repository, so this is the site.
const SITE_URL: &str = "https://yamp.cc";

/// The logo shown as the small badge on the corner of the cover. Discord loads it from the web, so
/// it is the copy in the repository and not a file of the app.
const BADGE_URL: &str = "https://raw.githubusercontent.com/Julius-IX/yamp/main/docs/assets/icons/mint-app-icon-transparent.png";

const APP_ID: Option<&str> = option_env!("YAMP_DISCORD_APP_ID");

/// The Discord application id of this build, when it has one.
pub fn app_id() -> Option<&'static str> {
  APP_ID.map(str::trim).filter(|id| !id.is_empty())
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
type Client = discord_rich_presence::DiscordIpcClient;
#[cfg(any(target_os = "android", target_os = "ios"))]
type Client = ();

/// Whether this build can show a presence by itself, which needs a desktop and a Discord
/// application id. Android also needs the SDK, which only its plugin can tell.
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
  /// Shown as the hover text of the cover, which is the one place of the card that has room for it.
  pub album: Option<&'a str>,
  pub start_ms: i64,
  /// Set when the length is known, which makes Discord draw a progress bar. Without it the card
  /// only counts up.
  pub end_ms: Option<i64>,
  /// The web page of the track and of its artist. Local files and live streams have none.
  pub url: Option<&'a str>,
  pub artist_url: Option<&'a str>,
}

/// What the card shows, after the limits of Discord were applied. Every desktop and Android
/// presence is made from one, so the two show the same.
#[derive(Debug, PartialEq)]
pub struct Card {
  /// The artist, which the status shows after "Listening to".
  pub name: String,
  pub details: String,
  pub state: String,
  pub details_url: Option<String>,
  pub state_url: Option<String>,
  pub cover: Option<String>,
  /// The hover text of the cover.
  pub hover: String,
  pub badge_url: &'static str,
  pub site_url: &'static str,
  /// A label and an address each, at most two, which is all Discord shows.
  pub buttons: Vec<(String, String)>,
  pub start_ms: i64,
  pub end_ms: Option<i64>,
}

impl Card {
  pub fn new(track: &Track) -> Self {
    let (details_url, state_url) = (web_url(track.url), web_url(track.artist_url));
    let mut buttons = Vec::new();
    if let Some(url) = details_url {
      buttons.push(("Listen on YouTube Music".to_string(), url.to_string()));
    }
    buttons.push(("Get yamp".to_string(), APP_URL.to_string()));
    Self {
      // the name is what the status shows after "Listening to", so it carries the artist
      name: field(track.artist),
      details: field(track.title),
      state: field(track.artist),
      details_url: details_url.map(str::to_string),
      state_url: state_url.map(str::to_string),
      cover: web_url(track.cover).map(str::to_string),
      // the hover text of the cover is the album when it is known, and otherwise where the music plays
      hover: track.album.map_or_else(|| "Listening on yamp".to_string(), field),
      badge_url: BADGE_URL,
      site_url: SITE_URL,
      buttons,
      start_ms: track.start_ms,
      end_ms: track.end_ms,
    }
  }
}

/// The connection to Discord, opened by the first `show`. It is dropped when a send fails, which
/// is what happens when Discord was closed, so the next `show` connects again.
#[derive(Clone, Default)]
pub struct Presence(Arc<Mutex<Option<Client>>>);

/// Discord refuses a line of less than 2 characters and of more than 128.
fn field(text: &str) -> String {
  let cut: String = text.chars().take(128).collect();
  if cut.chars().count() < 2 { format!("{cut}\u{200B}") } else { cut }
}

/// Discord only loads an image or a link from a web address, and a cached local cover has none.
/// A longer one is refused as well.
fn web_url(url: Option<&str>) -> Option<&str> {
  url.filter(|u| u.starts_with("http") && u.len() <= 256)
}

impl Presence {
  #[cfg(not(any(target_os = "android", target_os = "ios")))]
  pub fn show(&self, track: &Track) -> Result<(), Error> {
    use discord_rich_presence::{
      DiscordIpc, DiscordIpcClient,
      activity::{Activity, ActivityType, Assets, Button, StatusDisplayType, Timestamps},
    };

    let id = app_id().ok_or(Error::NotSetUp)?;
    let card = Card::new(track);
    let stamps = match card.end_ms {
      Some(end) => Timestamps::new().start(card.start_ms).end(end),
      None => Timestamps::new().start(card.start_ms),
    };
    let mut activity = Activity::new()
      .name(&card.name)
      .activity_type(ActivityType::Listening)
      .status_display_type(StatusDisplayType::Name)
      .details(&card.details)
      .state(&card.state)
      .timestamps(stamps);
    if let Some(url) = &card.details_url {
      activity = activity.details_url(url);
    }
    if let Some(url) = &card.state_url {
      activity = activity.state_url(url);
    }
    let mut assets = Assets::new()
      .large_text(&card.hover)
      .small_image(card.badge_url)
      .small_url(card.site_url)
      .small_text("yamp");
    if let Some(cover) = &card.cover {
      assets = assets.large_image(cover);
    }
    if let Some(url) = &card.details_url {
      assets = assets.large_url(url);
    }
    activity = activity.assets(assets).buttons(card.buttons.iter().map(|(label, url)| Button::new(label, url)).collect());

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

  fn track<'a>(url: Option<&'a str>, cover: Option<&'a str>) -> Track<'a> {
    Track { title: "Song", artist: "A", cover, album: None, start_ms: 1000, end_ms: Some(5000), url, artist_url: None }
  }

  #[test]
  fn card_carries_the_limits_and_the_links() {
    let card = Card::new(&track(Some("https://music.youtube.com/watch?v=x"), Some("/local/cover.jpg")));
    assert_eq!(card.state, "A\u{200B}");
    assert_eq!(card.hover, "Listening on yamp");
    assert_eq!(card.cover, None);
    assert_eq!(card.buttons.len(), 2);
    assert_eq!((card.start_ms, card.end_ms), (1000, Some(5000)));
  }

  #[test]
  fn card_without_a_page_only_offers_the_app() {
    let card = Card::new(&track(None, None));
    assert_eq!(card.details_url, None);
    assert_eq!(card.buttons, vec![("Get yamp".to_string(), APP_URL.to_string())]);
  }

  #[test]
  fn only_web_addresses_are_used() {
    assert_eq!(web_url(Some("https://i.example/a.jpg")), Some("https://i.example/a.jpg"));
    assert_eq!(web_url(Some("/home/user/cover.jpg")), None);
    assert_eq!(web_url(Some(&format!("https://{}", "a".repeat(300)))), None);
    assert_eq!(web_url(None), None);
  }
}
