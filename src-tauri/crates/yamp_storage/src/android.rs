//! Android backend. The Kotlin side in `./android` asks for `READ_MEDIA_AUDIO`, or the older
//! `READ_EXTERNAL_STORAGE` below Android 13.
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
