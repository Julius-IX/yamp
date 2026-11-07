//! Desktop backend. souvlaki publishes to MPRIS on Linux, the system transport controls on
//! Windows and Now Playing on macOS.
use crate::{ACTION_EVENT, Action, SEEK_STEP_SECS, Update};
use souvlaki::{
  MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig, SeekDirection,
};
use std::{
  ffi::c_void,
  sync::mpsc::{Receiver, Sender, channel},
  time::Duration,
};
use tauri::{AppHandle, Emitter, Wry, plugin::PluginApi};

/// The part of an update that is only sent again when it changes. Sending the metadata over
/// and over makes some desktops redraw the widget and drop the cover.
#[derive(PartialEq)]
struct Shown {
  title: String,
  artist: String,
  album: Option<String>,
  cover: Option<String>,
  duration_ms: Option<u64>,
}

pub fn start(app: &AppHandle<Wry>, _api: PluginApi<Wry, ()>) -> Result<Sender<Update>, Box<dyn std::error::Error>> {
  // a raw pointer cannot cross threads, so the handle travels as a number
  let hwnd = window_handle(app);
  let (tx, rx) = channel();
  let app = app.clone();
  // the controls are created on this thread and never leave it, some platforms are not Send
  std::thread::Builder::new()
    .name("media-session".into())
    .spawn(move || run(app, hwnd, rx))?;
  Ok(tx)
}

#[cfg(windows)]
fn window_handle(app: &AppHandle<Wry>) -> Option<usize> {
  use tauri::Manager;
  // the system transport controls are bound to a window on Windows
  app.get_webview_window("main")?.hwnd().ok().map(|h| h.0 as usize)
}

#[cfg(not(windows))]
fn window_handle(_app: &AppHandle<Wry>) -> Option<usize> {
  None
}

fn run(app: AppHandle<Wry>, hwnd: Option<usize>, rx: Receiver<Update>) {
  let config = PlatformConfig {
    display_name: "yamp",
    dbus_name: "yamp",
    hwnd: hwnd.map(|h| h as *mut c_void),
  };
  let mut controls = match MediaControls::new(config) {
    Ok(c) => c,
    Err(e) => return log::warn!("media controls are unavailable: {e:?}"),
  };
  let emitter = app.clone();
  let attached = controls.attach(move |event| {
    if let Some(action) = action(event) {
      let _ = emitter.emit(ACTION_EVENT, action);
    }
  });
  if let Err(e) = attached {
    return log::warn!("media controls could not attach: {e:?}");
  }

  let mut shown: Option<Shown> = None;
  while let Ok(update) = rx.recv() {
    let Some(title) = update.title else {
      shown = None;
      let _ = controls.set_playback(MediaPlayback::Stopped);
      let _ = controls.set_metadata(MediaMetadata::default());
      continue;
    };
    let next = Shown {
      title,
      artist: update.artist,
      album: update.album,
      cover: update.cover,
      duration_ms: update.duration_secs.filter(|d| *d > 0.0).map(|d| (d * 1000.0) as u64),
    };
    if shown.as_ref() != Some(&next) {
      let result = controls.set_metadata(MediaMetadata {
        title: Some(&next.title),
        artist: Some(&next.artist),
        album: next.album.as_deref(),
        cover_url: next.cover.as_deref(),
        duration: next.duration_ms.map(Duration::from_millis),
      });
      if let Err(e) = result {
        log::debug!("media metadata was not accepted: {e:?}");
      }
      shown = Some(next);
    }
    let progress = Some(MediaPosition(Duration::from_secs_f64(update.position_secs.max(0.0))));
    let playback = if update.playing { MediaPlayback::Playing { progress } } else { MediaPlayback::Paused { progress } };
    if let Err(e) = controls.set_playback(playback) {
      log::debug!("media playback state was not accepted: {e:?}");
    }
  }
}

fn action(event: MediaControlEvent) -> Option<Action> {
  let signed = |direction: SeekDirection, secs: f64| match direction {
    SeekDirection::Forward => secs,
    SeekDirection::Backward => -secs,
  };
  Some(match event {
    MediaControlEvent::Play => Action::Play,
    MediaControlEvent::Pause => Action::Pause,
    MediaControlEvent::Toggle => Action::Toggle,
    MediaControlEvent::Next => Action::Next,
    MediaControlEvent::Previous => Action::Previous,
    MediaControlEvent::Stop => Action::Stop,
    MediaControlEvent::Seek(d) => Action::SeekBy { secs: signed(d, SEEK_STEP_SECS) },
    MediaControlEvent::SeekBy(d, by) => Action::SeekBy { secs: signed(d, by.as_secs_f64()) },
    MediaControlEvent::SetPosition(MediaPosition(at)) => Action::SeekTo { secs: at.as_secs_f64() },
    // raising or quitting the window is not something a media widget needs to do here
    _ => return None,
  })
}
