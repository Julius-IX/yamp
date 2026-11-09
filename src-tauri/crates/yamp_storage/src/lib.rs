//! Access to the audio files of the device.
//!
//! Reading music from a folder needs a permission on Android, and the system asks for it from
//! native code, so the prompt has a Kotlin side in `./android`. Every other platform reads files
//! without asking. The folders themselves are picked and scanned by the app, see `yamp-db`.
use tauri::{
  Manager, Wry,
  plugin::{Builder, TauriPlugin},
};

#[cfg(target_os = "android")]
#[path = "android.rs"]
mod backend;
#[cfg(not(target_os = "android"))]
#[path = "none.rs"]
mod backend;

/// Managed by the plugin.
pub struct Storage {
  inner: backend::Access,
}

impl Storage {
  /// Asks for the permission to read audio files, unless it was given before, and returns whether
  /// it is granted. The call waits for the answer of the person, so it must not run on the main
  /// thread, where the prompt itself is shown from.
  pub fn request_audio_access(&self) -> bool {
    self.inner.request()
  }
}

pub fn init() -> TauriPlugin<Wry> {
  Builder::new("yamp-storage")
    .setup(|app, api| {
      app.manage(Storage { inner: backend::start(app, api)? });
      Ok(())
    })
    .build()
}
