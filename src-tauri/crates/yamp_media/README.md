# yamp-media

Backend-agnostic music source interfaces, the yt-dlp backend and the download queue. It has no Tauri dependency, so `cargo test` here builds only this crate.
The app does not use it directly, `yt-api` is the surface.

| Part | Covers |
| ---- | ------ |
| `MediaSource` | search, track metadata, playlists, related tracks, stream URLs |
| `Downloader` | saving a track to disk with progress |
| `Updater` | install state and updating the provider tooling |
| `Backend` | the three parts held together, from one provider or several |
| `DownloadQueue` | first come first served downloads with a parallel cap |

`ytdlp::YtDlpBackend<E>` holds the provider logic and delegates process execution to a `YtDlpEngine`. `DesktopEngine` is here, the Android engine lives in `yt-api` because it needs Tauri.

## Adding a backend

Implement the three traits for a new type and return `Backend::single(it)` from `yt_api::platform`, or implement `YtDlpEngine` for a new platform.
