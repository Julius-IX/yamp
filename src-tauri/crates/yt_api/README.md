# yt-api

The only crate the app needs for music sources. It is a Tauri plugin that registers a `YtApi` state, picks the backend for the platform, keeps it installed and owns the download queue.

```rust
tauri::Builder::default().plugin(yt_api::init())
// later, in a command
api.search("query", None).await?;
api.enqueue("id", AudioQuality::High);
api.update().await?;
```

| Area | Methods |
| ---- | ------- |
| Reading | `search`, `track`, `playlist`, `related`, `stream` |
| Downloads | `enqueue`, `cancel`, `jobs`, `clear_finished` |
| Tooling | `status`, `update` |

Every queue change is emitted as the `yt-api:download` event with a `DownloadJob`. Errors are logged through the `log` facade, the app decides where that goes.

## Platforms

`platform.rs` is the only place that maps a platform to an engine.

| Platform | Engine |
| -------- | ------ |
| Linux, Windows, macOS | yt-dlp binary in the app data dir, downloaded and checksum verified |
| Android | youtubedl-android through the Kotlin plugin in `android/` |
| iOS | none yet, the build stops with a clear error |
