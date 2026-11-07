//! The playing track is published here. Playback itself lives in the webview.
use tauri::{AppHandle, State};
use yamp_session::{Session, Update};

/// Mirrors the player to the system's media controls and the window title. A missing `title`
/// means nothing is playing.
#[tauri_groups::command(session)]
pub fn update(
  app: AppHandle,
  session: State<'_, Session>,
  title: Option<String>,
  artist: String,
  album: Option<String>,
  cover: Option<String>,
  duration_secs: Option<f64>,
  position_secs: f64,
  playing: bool,
) {
  // The title is set here and not from the webview, which needs a capability for it and
  // swallowed the refusal without a trace.
  #[cfg(desktop)]
  {
    use tauri::Manager;
    let label = match (&title, artist.is_empty()) {
      (Some(t), false) => format!("{t} · {artist}"),
      (Some(t), true) => t.clone(),
      (None, _) => "yamp".into(),
    };
    if let Some(window) = app.get_webview_window("main") {
      if let Err(e) = window.set_title(&label) {
        log::warn!("window title was not set: {e}");
      }
    }
  }
  #[cfg(not(desktop))]
  let _ = app;
  session.update(Update { title, artist, album, cover, duration_secs, position_secs, playing });
}
