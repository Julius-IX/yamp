//! Android engine. yt-dlp runs inside youtubedl-android, an embedded Python build that updates
//! itself. The Kotlin side and its Gradle dependency live in `./android` and are wired into the
//! app's Gradle build by the Tauri CLI.
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use tauri::{
  ipc::{Channel, InvokeResponseBody},
  plugin::{PluginApi, PluginHandle},
  Wry,
};
use yamp_media::{
  ytdlp::{YtDlpArgs, YtDlpEngine},
  MediaError, ProgressSink, Result,
};

/// Set once while the plugin registers, which is the only moment Tauri hands the handle out.
static HANDLE: OnceLock<PluginHandle<Wry>> = OnceLock::new();

/// Registers the Kotlin plugin and returns the engine that talks to it.
pub fn engine(api: &PluginApi<Wry, ()>) -> std::result::Result<AndroidEngine, Box<dyn std::error::Error>> {
  let handle = api.register_android_plugin("com.salad.yamp.ytdlp", "YtdlpPlugin")?;
  let _ = HANDLE.set(handle);
  Ok(AndroidEngine)
}

#[derive(Serialize)]
struct ExecuteArgs<'a> {
  #[serde(flatten)]
  args: &'a YtDlpArgs,
  channel: Option<Channel<serde_json::Value>>,
}

#[derive(Deserialize)]
struct Out {
  out: String,
}

#[derive(Deserialize)]
struct Percent {
  percent: f32,
}

async fn call(command: &'static str, payload: impl Serialize) -> Result<String> {
  let handle = HANDLE.get().ok_or_else(|| MediaError::Engine("yt-dlp plugin not ready".into()))?;
  handle
    .run_mobile_plugin_async::<Out>(command, payload)
    .await
    .map(|o| o.out)
    .map_err(|e| MediaError::Engine(e.to_string()))
}

pub struct AndroidEngine;

#[async_trait]
impl YtDlpEngine for AndroidEngine {
  async fn run(&self, args: &YtDlpArgs, progress: Option<ProgressSink>) -> Result<String> {
    log::debug!("yt-dlp {}", args.target);
    // The channel must outlive the call, Kotlin keeps sending on it until `execute` returns.
    let channel = progress.map(|sink| {
      Channel::<serde_json::Value>::new(move |body| {
        if let InvokeResponseBody::Json(json) = body {
          if let Ok(p) = serde_json::from_str::<Percent>(&json) {
            sink(p.percent);
          }
        }
        Ok(())
      })
    });
    call("execute", ExecuteArgs { args, channel }).await
  }

  async fn version(&self) -> Result<Option<String>> {
    let v = call("version", serde_json::json!({})).await?;
    Ok((!v.is_empty()).then_some(v))
  }

  async fn update(&self) -> Result<String> {
    log::info!("updating yt-dlp through youtubedl-android");
    call("update", serde_json::json!({})).await
  }
}
