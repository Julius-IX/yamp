//! Platforms that read files without a permission.
use tauri::{AppHandle, Wry, plugin::PluginApi};

pub struct Access;

pub fn start(_app: &AppHandle<Wry>, _api: PluginApi<Wry, ()>) -> Result<Access, Box<dyn std::error::Error>> {
  Ok(Access)
}

impl Access {
  pub fn request(&self) -> bool {
    true
  }
}
