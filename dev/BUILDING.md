# Building yamp

How to run and package the app. Each crate under `src-tauri/crates` has its own readme with the
details of its part, this file only shows the minimal setup and points there.

For running tests see [TESTING.md](TESTING.md).
Use [build.sh](../scripts/build.sh) to create release builds with ease.

## Layout

| Path                  | Holds                                                              |
| --------------------- | ------------------------------------------------------------------ |
| [`src`](../src)                           | The frontend, React and MUI on Vite                            |
| [`src-tauri`](../src-tauri)               | The Tauri app and the Cargo workspace                          |
| [`src-tauri/crates`](../src-tauri/crates) | The workspace crates, most of them without a Tauri dependency  |
| [`src-tauri/migrations`](../src-tauri/migrations) | The SQLite schema, see [`yamp-db`](../src-tauri/crates/yamp_db/README.md#schema) |
| [`src-tauri/.sqlx`](../src-tauri/.sqlx)   | Query data for builds without a database, committed            |
| [`src-tauri/gen/android`](../src-tauri/gen/android) | The Android project, built by Gradle                 |

## Prerequisites

- Node.js with npm.
- Rust through `rustup`. The workspace uses edition 2024, so the toolchain has to be recent.
- The system libraries of Tauri for the platform, see
  <https://v2.tauri.app/start/prerequisites/>.
- `sqlx-cli`, for changing queries or the schema only, see
  [Changing a query or the schema](../src-tauri/crates/yamp_db/README.md#changing-a-query-or-the-schema):
  `cargo install sqlx-cli --no-default-features --features sqlite`.

```sh
npm ci
```

## First build

[`src-tauri/.env`](../src-tauri/.env) sets `DATABASE_URL` to `sqlite:dev/dev-db.sqlite`, and the `sqlx` macros read it.
With it set, a build checks every query against that database, and a fresh clone has none. There
are two ways out, run from `src-tauri`:

```sh
# use the committed query data, nothing to set up
export SQLX_OFFLINE=true

# or create the development database
mkdir -p dev && sqlx database setup
```

`SQLX_OFFLINE=true` is also what CI should set. The readme of
[`yamp-db`](../src-tauri/crates/yamp_db/README.md#changing-a-query-or-the-schema) explains changing
queries and refreshing `.sqlx`, and [TESTING.md](TESTING.md#the-database) has the commands.

## Running

```sh
npm run tauri dev             # desktop, with hot reload for the frontend
npm run tauri android dev     # a phone or an emulator
```

A debug desktop build writes [`src/lib/commands.ts`](../src/lib/commands.ts) and
[`src/lib/types.ts`](../src/lib/types.ts) when it starts, from the commands and types in Rust, see
[`tauri-groups`](../src-tauri/crates/tauri_groups/README.md). Both files are committed, so a change to a command or a type shows up
in `git diff` after the first `tauri dev` and goes into the same commit.

The frontend alone can be checked without Rust:

```sh
npx tsc --noEmit
npm run build
```

## Build time configuration

Some features exist only in a build that has a key for them. Without one the feature is left out
and the settings do not show it. A key is read from, the first one with a value winning:

1. the environment of the build, which is what CI sets,
2. `config.toml` in the folder of the crate that owns it, copied from the `config.toml.example`
   next to it,
3. the `[env]` table of `src-tauri/.cargo/config.toml`.

Both `config.toml` and `.cargo/config.toml` are ignored by git. An empty value counts as not set.

### Discord Rich Presence

Make an application in the Discord developer portal. Its name is shown on the card, and its id goes
into the build:

```sh
cp src-tauri/crates/yamp_discord/config.toml.example src-tauri/crates/yamp_discord/config.toml
# set YAMP_DISCORD_APP_ID in it
```

The template is [`config.toml.example`](../src-tauri/crates/yamp_discord/config.toml.example). See
[Which builds have it](../src-tauri/crates/yamp_discord/README.md#which-builds-have-it) and
[When nothing shows up](../src-tauri/crates/yamp_discord/README.md#when-nothing-shows-up) in the
readme of [`yamp-discord`](../src-tauri/crates/yamp_discord/README.md).

### Cloud sync

The sign in for Google Drive and OneDrive needs a client id per platform. A service without an id
for the platform being built is not offered.

```sh
cp src-tauri/crates/cloud_auth/config.toml.example src-tauri/crates/cloud_auth/config.toml
```

The template is [`config.toml.example`](../src-tauri/crates/cloud_auth/config.toml.example).

```toml
[env]
YAMP_GOOGLE_DESKTOP_CLIENT_ID = "..."
YAMP_GOOGLE_DESKTOP_CLIENT_SECRET = "..."
YAMP_GOOGLE_ANDROID_CLIENT_ID = "..."
YAMP_MICROSOFT_DESKTOP_CLIENT_ID = "..."
```

`DESKTOP` covers Linux, macOS and Windows, `MOBILE` covers Android and iOS, and a variable of one
system, such as `LINUX`, wins over its generic one. The readme of
[`cloud-auth`](../src-tauri/crates/cloud_auth/README.md) has the pieces:

- [Which services a build has](../src-tauri/crates/cloud_auth/README.md#which-services-a-build-has),
  every level of the variables.
- [Google Drive](../src-tauri/crates/cloud_auth/README.md#google-drive) and
  [OneDrive](../src-tauri/crates/cloud_auth/README.md#onedrive), how to register each client.
- [The scheme of the app](../src-tauri/crates/cloud_auth/README.md#the-scheme-of-the-app), the deep
  link of the mobile builds.
- [When a service is missing](../src-tauri/crates/cloud_auth/README.md#when-a-service-is-missing),
  reading the reason in the log.

### Updates

The desktop updater is off until the app has a public key. Generate a pair once:

```sh
npx tauri signer generate -w ~/.tauri/yamp.key
```

Put the content of `yamp.key.pub` in `plugins.updater.pubkey` of
[`src-tauri/tauri.conf.json`](../src-tauri/tauri.conf.json). It is not a secret.
The private key signs the bundles, so a release build needs it in the environment. It cannot go in
`src-tauri/.cargo/config.toml`: the Tauri CLI does the signing, and cargo's `[env]` only reaches
the processes cargo starts. `scripts/build.sh` reads `~/.tauri/yamp.key` (or `YAMP_SIGNING_KEY`)
itself. Without the script:

```sh
export TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/yamp.key)"   # the content or the path of the file
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="..."               # only when the key has one
```

Losing the private key means every installed copy has to be replaced by hand once, since an app
only trusts the key it shipped with. Android does not use this key, see below.

## Release builds

Raise `version` in [`src-tauri/tauri.conf.json`](../src-tauri/tauri.conf.json) first, an update is
found by comparing it. Keep [`package.json`](../package.json) and
[`src-tauri/Cargo.toml`](../src-tauri/Cargo.toml) in step. The module docs of
[`updater.rs`](../src-tauri/src/api/updater.rs) say how the updater is wired.

### Linux, AppImage

```sh
npm run tauri build -- --bundles appimage
```

Only an AppImage can replace itself, any other Linux install hides the update option.

### Windows, NSIS

```sh
npm run tauri build -- --bundles nsis
```

### Android, universal APK

The Gradle project reads the signing key of the release from `src-tauri/gen/android/keystore.properties`,
which is ignored by git, see
[`build.gradle.kts`](../src-tauri/gen/android/app/build.gradle.kts). The build script reads it
unconditionally, so a debug build needs the file as well.

```properties
keyAlias=...
password=...
storeFile=/absolute/path/to/release.jks
```

```sh
npm run tauri android build -- --apk
```

The result is `src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release.apk`.
Android installs an update over an existing app only when both are signed with the same keystore,
so the keystore has to stay the same for every release.

The deep link of the app for the sign in is also kept in the
[`AndroidManifest.xml`](../src-tauri/gen/android/app/src/main/AndroidManifest.xml), see
[the scheme of the app](../src-tauri/crates/cloud_auth/README.md#the-scheme-of-the-app).

The Android toolchain needs the SDK, the NDK, `JAVA_HOME`, `ANDROID_HOME` and `NDK_HOME`, and the
Rust targets:

```sh
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
```

### Publishing

A release on GitHub carries the bundles, their `.sig` files and a `latest.json` that lists the URL
and signature of every platform. `tauri-action` writes `latest.json`. The Android app looks at the
newest release and opens its `.apk` asset. macOS and iOS are not built here.

## Crate readmes

| Crate | Read it for |
| ----- | ----------- |
| [`yamp-db`](../src-tauri/crates/yamp_db/README.md) | Schema, migrations, `sqlx` setup, linking devices |
| [`yt-api`](../src-tauri/crates/yt_api/README.md) | The engine of each platform and the Android plugin |
| [`yamp-media`](../src-tauri/crates/yamp_media/README.md) | Music source traits, the yt-dlp backend and the download queue |
| [`cloud-auth`](../src-tauri/crates/cloud_auth/README.md) | Client registration, variables and the mobile deep link |
| [`cloud-sync`](../src-tauri/crates/cloud_sync/README.md) | Providers and the storage interface |
| [`cloud-kit`](../src-tauri/crates/cloud_kit/README.md) | Documents, logs and blobs on top of a store |
| [`yamp-discord`](../src-tauri/crates/yamp_discord/README.md) | The presence card and its application id |
| [`tauri-groups`](../src-tauri/crates/tauri_groups/README.md) | Command groups and the generated bindings |
| [`tauri-settings`](../src-tauri/crates/tauri_settings/README.md) | Stored settings |
| [`ts-types`](../src-tauri/crates/ts_types/README.md) | The derive that describes a Rust type for the bindings |
