//! Android backend. The Kotlin side in `./android` sends the person to the settings page for access
//! to all files, or asks for `READ_EXTERNAL_STORAGE` below Android 11. The reason is in there.
use serde::Deserialize;
use tauri::{
  AppHandle, Wry,
  plugin::{PluginApi, PluginHandle},
};

#[derive(Deserialize)]
struct Answer {
  granted: bool,
}

pub struct Access(PluginHandle<Wry>);

pub fn start(_app: &AppHandle<Wry>, api: PluginApi<Wry, ()>) -> Result<Access, Box<dyn std::error::Error>> {
  Ok(Access(api.register_android_plugin("com.salad.yamp.storage", "StoragePlugin")?))
}

impl Access {
  pub fn request(&self) -> bool {
    match self.0.run_mobile_plugin::<Answer>("request", ()) {
      Ok(answer) => answer.granted,
      Err(e) => {
        log::warn!("the audio permission could not be asked for: {e}");
        false
      }
    }
  }
}
