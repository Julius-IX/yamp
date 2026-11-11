mod api;
mod library;
mod logging;

use tauri::Manager;

/// Every saved setting. Keys outside this list are dropped when the settings file is loaded.
const SETTINGS: &[&str] = &[
  "theme",
  "mode",
  "volume",
  "muted",
  "shuffle",
  "repeat",
  "autoplay",
  "quality",
  "playback",
  "history",
  "recentSearches",
  "animations",
  "tooltips",
  "statsEnabled",
  "statsDevice",
  "cloudSections",
  "customThemes",
  "discordPresence",
  "prefetch",
  "prefetchAmount",
  "engineAutoUpdate",
  "engineUpdatedMs",
  "updateCheck",
  "updateSkipped",
  "lastCompact",
  "country",
  "clock24",
  "termsAccepted",
  library::SYNC_KEY,
];

/// The AppImage bundles GStreamer, but the helper that scans plugins is not always found at the
/// path GStreamer expects. The web process then warns on every media element it creates. Without
/// the helper the scan runs in process, which is slower only on the first start.
#[cfg(target_os = "linux")]
fn scan_gstreamer_plugins_in_process() {
  use std::{env::var_os, path::Path};
  if var_os("APPDIR").is_none() || var_os("GST_REGISTRY_FORK").is_some() {
    return;
  }
  if var_os("GST_PLUGIN_SCANNER").is_some_and(|p| Path::new(&p).is_file()) {
    return;
  }
  // SAFETY: runs first in `run`, before the web view or any thread of the app exists.
  unsafe { std::env::set_var("GST_REGISTRY_FORK", "no") };
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  #[cfg(target_os = "linux")]
  scan_gstreamer_plugins_in_process();
  logging::init();

  #[cfg(all(debug_assertions, desktop))]
  export_bindings();

  let builder = tauri::Builder::default()
    .plugin(tauri_plugin_opener::init())
    // the folder picker for the local music folders of the data settings
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_fs::init())
    .plugin(yt_api::init())
    .plugin(yamp_session::init())
    .plugin(yamp_storage::init());
  // a sign in on a phone is answered with a link of a scheme of this app, which the system hands over
  #[cfg(any(target_os = "android", target_os = "ios"))]
  let builder = builder.plugin(tauri_plugin_deep_link::init());

  builder
    .setup(|app| {
      tauri_settings::init(app.handle(), SETTINGS)?;
      logging::attach(app.handle().clone());
      app.manage(api::Presence::default());
      api::init_updater(app)?;
      library::init(app.handle())?;
      #[cfg(any(target_os = "android", target_os = "ios"))]
      {
        use tauri_plugin_deep_link::DeepLinkExt;
        app.deep_link().on_open_url(|event| {
          for url in event.urls() {
            // the query carries the authorization code, so only the part before it is logged
            let taken = cloud_auth::deliver_redirect(url.as_str());
            let (scheme, host, path) = (url.scheme(), url.host_str().unwrap_or(""), url.path());
            log::info!("link opened the app: {scheme}://{host}{path}, taken by a sign in: {taken}");
          }
        });
      }
      log::info!("yamp started");
      Ok(())
    })
    .invoke_handler(tauri_groups::handler())
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

#[cfg(any(all(debug_assertions, desktop), test))]
fn export_bindings() {
  tauri_groups::export_ts(concat!(env!("CARGO_MANIFEST_DIR"), "/../src/lib"))
    .expect("failed to write src/lib/{commands,types}.ts");
}
