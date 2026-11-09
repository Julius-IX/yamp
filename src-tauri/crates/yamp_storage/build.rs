// No JS facing commands, the app exposes its own typed one on top of `Storage`.
const COMMANDS: &[&str] = &[];

fn main() {
  tauri_plugin::Builder::new(COMMANDS).android_path("android").build();
}
