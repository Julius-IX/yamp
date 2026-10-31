# tauri-settings

A small key-value store for UI state, saved as one JSON file in the app data directory.

## Usage

Register the store in `setup` with the keys the app has:

```rust
const SETTINGS: &[&str] = &["theme", "mode"];

tauri::Builder::default()
  .setup(|app| {
    tauri_settings::init(app.handle(), SETTINGS)?;
    Ok(())
  })
  .invoke_handler(tauri_groups::handler())
```

Rust code reads and writes through the managed state:

```rust
fn read(settings: State<Settings>) -> Option<String> {
  settings.get::<String>("theme")
}
```

The frontend gets two commands in the `settings` group, `getSettings` and `setSetting`. They are picked up by `tauri-groups` and show up in the generated `commands.ts`.

## Keys

Only the keys passed to `init` are kept. Anything else is dropped when the file is loaded, and saving an unknown key is an error. Defaults are not stored, a setting that was never changed is absent and the reader supplies the default.

## Early reads

`peek` reads one key from a settings file without touching it. `peek_early` does the same for the file of an app identifier on Linux, for choices that have to be made before the Tauri app exists.

## Features

| Feature | Adds |
| ------- | ---- |
| `tauri` (default) | `init` and the `settings` commands |

Without `tauri` only `Settings`, `peek` and `peek_early` are built, which needs no Tauri dependency.

## Files

A save writes `settings.json.tmp` and renames it over `settings.json`. A file that is not a JSON object is moved to `settings.json.corrupt` and the store starts empty.
