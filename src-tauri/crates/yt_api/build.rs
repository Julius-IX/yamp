// No JS facing commands, the app exposes its own typed ones on top of `YtApi`.
const COMMANDS: &[&str] = &[];

fn main() {
  tauri_plugin::Builder::new(COMMANDS).android_path("android").build();
}
