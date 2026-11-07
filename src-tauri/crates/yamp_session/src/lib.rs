//! Publishes the playing track to the operating system, so media keys, headphone buttons, lock
//! screens and quick settings can show it and control it.
//!
//! The webview cannot do this itself. WebKitGTK does not expose its media session on Linux and
//! Android's WebView never creates a media notification, so each platform gets a native backend
//! behind the same [`Session`]. Desktop uses souvlaki (MPRIS, SMTC, Now Playing) and Android a
//! Kotlin `MediaSession` with a foreground service. Presses come back as [`ACTION_EVENT`].
//!
//! Updates go through a channel to one thread per backend. The calls into the system can block,
//! and a command must not wait for them. The channel also keeps the updates in order.
use serde::{Deserialize, Serialize};
use std::sync::mpsc::Sender;
use tauri::{
  Manager, Wry,
  plugin::{Builder, TauriPlugin},
};

#[cfg(target_os = "android")]
#[path = "android.rs"]
mod backend;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[path = "desktop.rs"]
mod backend;
#[cfg(target_os = "ios")]
#[path = "none.rs"]
mod backend;

/// Event carrying an [`Action`] whenever the system asks for one.
pub const ACTION_EVENT: &str = "yamp-session:action";

/// Step of a seek button that does not say how far to go.
pub(crate) const SEEK_STEP_SECS: f64 = 10.0;

/// What is playing. `title` is `None` when nothing is, which clears the system's controls.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Update {
  pub title: Option<String>,
  pub artist: String,
  pub album: Option<String>,
  /// An http(s) URL. Each system fetches it on its own.
  pub cover: Option<String>,
  pub duration_secs: Option<f64>,
  pub position_secs: f64,
  pub playing: bool,
}

/// A press on the system's controls. Play and pause are separate from toggle because a headset
/// button says which one it means and acting on the wrong one would flip the state back.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum Action {
  Play,
  Pause,
  Toggle,
  Next,
  Previous,
  Stop,
  SeekBy { secs: f64 },
  SeekTo { secs: f64 },
}

/// Managed by the plugin. Dropping the sender ends the backend thread.
pub struct Session {
  tx: Sender<Update>,
}

impl Session {
  pub fn update(&self, update: Update) {
    if self.tx.send(update).is_err() {
      log::warn!("the media session backend is not running");
    }
  }
}

/// Starts the backend and manages [`Session`]. Add it to the builder before the first update.
pub fn init() -> TauriPlugin<Wry> {
  Builder::new("yamp-session")
    .setup(|app, api| {
      let tx = backend::start(app, api)?;
      app.manage(Session { tx });
      log::info!("media session ready");
      Ok(())
    })
    .build()
}
