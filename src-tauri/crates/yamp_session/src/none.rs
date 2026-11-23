//! Platforms without a native backend. The webview's own media session stays in charge there.
use crate::Update;
use std::sync::mpsc::{Sender, channel};
use tauri::{AppHandle, Wry, plugin::PluginApi};

pub fn start(_app: &AppHandle<Wry>, _api: PluginApi<Wry, ()>) -> Result<Sender<Update>, Box<dyn std::error::Error>> {
  // updates are accepted and dropped, so a caller never has to know
  let (tx, rx) = channel::<Update>();
  std::thread::spawn(move || while rx.recv().is_ok() {});
  Ok(tx)
}

pub async fn native(_command: &str, _payload: serde_json::Value) -> Result<serde_json::Value, String> {
  Err("there is no native side on this platform".into())
}
