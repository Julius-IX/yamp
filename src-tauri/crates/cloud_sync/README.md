# cloud-sync

Stores files on the user's own cloud account. Meant as the transport for syncing data between devices, so it only moves bytes and knows nothing about what they contain. Higher level shapes (documents, logs, big blobs) live in `cloud-kit`.

## Provider

The service is a runtime choice, since devices only sync when they use the same account of the same service.

| Provider | Platforms | Syncs with |
| -------- | --------- | ---------- |
| Google Drive | all | any device |
| OneDrive | all | any device |
| iCloud | Apple targets only | Apple devices only |

iCloud has no API outside Apple platforms. Syncing Android with iOS or Linux with macOS therefore needs Google Drive or OneDrive on every device. `Provider::available()` lists what a build can use and `Provider::cross_platform()` tells a settings screen which ones to offer when the user owns mixed devices. `Provider` serializes to a stable name, so the choice can be stored and synced.

## Usage

```rust
use cloud_sync::{CloudStore, Provider, Settings};

let store = cloud_sync::open(Provider::GoogleDrive, Settings::default().with_tokens(tokens))?;
store.put("wal/device-1/0001", &bytes)?;
let segments = store.list("wal/")?;
```

`tokens` implements `TokenProvider`. The crate runs no login flow, so the consent screen and the token storage stay with the application. iCloud takes `with_container(dir)` instead, the app's ubiquity container.

## Model

A flat store of `key -> bytes` inside a private folder of the account (Drive `appDataFolder`, OneDrive `approot`, the iCloud container). Slashes in keys only group them for `list`.

- There are no conditional writes. Google Drive offers none, so each key should have a single writer, for example by putting a device id into it.
- `revision` changes with the content and is only comparable with an earlier value of the same key.
- Objects are limited to `MAX_OBJECT_SIZE` (4 MiB) so a single request upload is enough.
- All calls block. Async code runs them on a blocking thread.
- Keys are validated the same on every backend, see `validate_key`. Lowercase keys are advised, since OneDrive and macOS ignore case.

## iCloud

Files that are not downloaded show up as `.<name>.icloud` stubs. They are listed, but `get` returns `Error::NotDownloaded` until the download is started through the native API.
