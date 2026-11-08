mod api;
mod library;
mod logging;

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
  "prefetch",
  "lastCompact",
  "country",
  library::SYNC_KEY,
];

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  logging::init();

  #[cfg(all(debug_assertions, desktop))]
  export_bindings();

  let builder = tauri::Builder::default()
    .plugin(tauri_plugin_opener::init())
    .plugin(yt_api::init())
    .plugin(yamp_session::init());
  // a sign in on a phone is answered with a link of a scheme of this app, which the system hands over
  #[cfg(any(target_os = "android", target_os = "ios"))]
  let builder = builder.plugin(tauri_plugin_deep_link::init());

  builder
    .setup(|app| {
      tauri_settings::init(app.handle(), SETTINGS)?;
      logging::attach(app.handle().clone());
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
