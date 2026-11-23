//! Android backend. The Kotlin side in `./android` owns a `MediaSession` and a media style
//! notification, which is what quick settings, the lock screen and headsets work with. The
//! notification runs in a foreground service so playback survives the app leaving the screen.
use crate::{ACTION_EVENT, Update};
use serde::Serialize;
use std::sync::{
  OnceLock,
  mpsc::{Sender, channel},
};
use tauri::{
  AppHandle, Emitter, Wry,
  ipc::{Channel, InvokeResponseBody},
  plugin::{PluginApi, PluginHandle},
};

/// Set once while the plugin registers, which is the only moment Tauri hands the handle out.
static HANDLE: OnceLock<PluginHandle<Wry>> = OnceLock::new();

/// The commands of the Kotlin plugin that the app may call besides the media session. A list keeps
/// a typo from making a trip over the bridge to be rejected there.
const COMMANDS: &[&str] = &["discord"];

#[derive(Serialize)]
struct ListenArgs {
  channel: Channel<serde_json::Value>,
}

pub fn start(app: &AppHandle<Wry>, api: PluginApi<Wry, ()>) -> Result<Sender<Update>, Box<dyn std::error::Error>> {
  let handle = api.register_android_plugin("com.salad.yamp.session", "SessionPlugin")?;
  let _ = HANDLE.set(handle.clone());
  let emitter = app.clone();
  let (tx, rx) = channel::<Update>();
  // The plugin calls block until Kotlin answers, so none of them is made from the setup that
  // runs on the main thread, where Kotlin may be waiting for the same thread.
  std::thread::Builder::new().name("media-session".into()).spawn(move || {
    // the actions arrive as JSON objects that already have the shape of `Action`
    let channel = Channel::<serde_json::Value>::new(move |body| {
      if let InvokeResponseBody::Json(json) = body {
        if let Ok(action) = serde_json::from_str::<serde_json::Value>(&json) {
          let _ = emitter.emit(ACTION_EVENT, action);
        }
      }
      Ok(())
    });
    if let Err(e) = handle.run_mobile_plugin::<serde_json::Value>("listen", ListenArgs { channel }) {
      log::warn!("media session could not listen for actions: {e}");
    }
    while let Ok(update) = rx.recv() {
      if let Err(e) = handle.run_mobile_plugin::<serde_json::Value>("update", &update) {
        log::debug!("media session update failed: {e}");
      }
    }
  })?;
  Ok(tx)
}

pub async fn native(command: &str, payload: serde_json::Value) -> Result<serde_json::Value, String> {
  let command = COMMANDS.iter().find(|c| **c == command).ok_or_else(|| format!("unknown native command {command}"))?;
  let handle = HANDLE.get().ok_or("the media session is not ready")?;
  handle.run_mobile_plugin_async::<serde_json::Value>(command, payload).await.map_err(|e| e.to_string())
}
