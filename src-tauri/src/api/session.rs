//! The playing track is published here. Playback itself lives in the webview.
use tauri::{AppHandle, State};
use yamp_session::{Session, Update};

/// Mirrors the player to the system's media controls and the window title. A missing `title`
/// means nothing is queued. The window is only named after the track while it plays, so a taskbar
/// or a status bar that reads the title shows what is heard and falls back to the app name when
/// nothing is.
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
    let label = match (&title, playing, artist.is_empty()) {
      (Some(t), true, false) => format!("{t} · {artist}"),
      (Some(t), true, true) => t.clone(),
      _ => "yamp".into(),
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
