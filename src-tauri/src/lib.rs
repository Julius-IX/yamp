mod api;
mod logging;

/// Every saved setting. Keys outside this list are dropped when the settings file is loaded.
const SETTINGS: &[&str] = &["theme", "mode"];

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  logging::init();

  #[cfg(all(debug_assertions, desktop))]
  export_bindings();

  tauri::Builder::default()
    .plugin(tauri_plugin_opener::init())
    .plugin(yt_api::init())
    .setup(|app| {
      tauri_settings::init(app.handle(), SETTINGS)?;
      logging::attach(app.handle().clone());
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
