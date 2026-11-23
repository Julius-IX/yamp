# yamp-discord

Discord Rich Presence, the "listening to" card on the Discord profile of the user. It has no Tauri
dependency, so `cargo test` here builds only this crate. The app reaches it through the `discord`
command group in `src/api/discord.rs`.

| Platform | How the card gets to Discord                                                              |
| -------- | ----------------------------------------------------------------------------------------- |
| Desktop  | This crate, over the local socket of the Discord app that runs on the computer            |
| Android  | The Discord Social SDK in the session plugin, see [Android](#android)                     |
| iOS      | Nothing. Discord only allows a presence there after a sign in, which is not built         |

```rust
let presence = Presence::default(); // cheap to clone, clones share the connection

presence.show(&Track {
  title: "Song",
  artist: "Artist",
  cover: Some("https://example.com/cover.jpg"),
  start_ms,
  end_ms: Some(end_ms),
  url: Some("https://music.youtube.com/watch?v=abc"),
  artist_url: None,
})?;

presence.hide();
```

The connection to Discord is opened by the first `show`. It is dropped when a send fails, which
is what happens when Discord was closed, so the next `show` connects again. A call is blocking
and can take a moment, so the app runs it off the main thread.

Only a desktop can talk to Discord over a local socket. On Android and iOS `Presence` does
nothing, and the dependency on `discord-rich-presence` is not compiled in at all. The text and the
links of the card are made by `Card::new`, which desktop and Android both use, so the two show the
same thing. Android takes the finished `Card` and hands it to the SDK.

## Which builds have it

The Discord application that owns the card is made once in the
[developer portal](https://discord.com/developers/applications). Its id is read when the crate is
compiled, and **a build without an id has no presence**. `available()` returns false, and the
settings do not show the option. The name of the application is shown by Discord in the
places the card does not override.

Copy `config.toml.example` to `config.toml` in this folder and fill in `YAMP_DISCORD_APP_ID`. The
build script reads it from there, so it does not matter which folder cargo, `tauri dev` or
Android Studio is started in. These are the places it looks, the first one with a value wins:

1. the environment of the build, which is what a CI build sets,
2. `config.toml` in this folder,
3. the `[env]` table of `src-tauri/.cargo/config.toml`.

An empty value counts as not set. Change the value and the next build picks it up.

**Do not commit `config.toml`.** The id is not a secret, every build carries it. But a fork that
builds with it shows up as this application on the profile of its users. A fork makes its own.
The `.gitignore` of this folder keeps the file out, and only the `.example` file is committed.

## Android

A phone has no socket to talk to, so the card is shown by the
[Discord Social SDK](https://docs.discord.com/developers/discord-social-sdk/overview), which talks
to the Discord app of the phone. Rich Presence needs no sign in and no account linking for this,
only a Discord app that is installed and signed in. That is what the SDK offers from version 1.10 on.

**The SDK is not in the repository.** It is a download from Discord, so every builder gets it once.
**A build without it compiles as usual and has no presence on Android**,
and the settings say so. Nothing else changes.

### Setup

1. Make the Discord application and put its id in `config.toml`, as described in
   [Which builds have it](#which-builds-have-it). Android reads the same id as the desktop does.
2. Download the Android package of the Discord Social SDK, version 1.10 or newer. The download is
   described in the
   [getting started guide](https://docs.discord.com/developers/discord-social-sdk/getting-started/using-c++)
   of the SDK. It contains `discord_partner_sdk.aar`.
3. Copy the file to `src-tauri/crates/yamp_session/android/libs/discord_partner_sdk.aar`. The
   folder is ignored by git, **do not commit the file**.
4. Install **CMake 3.22 or newer** in the Android SDK Manager, next to the NDK. The glue is a small
   C++ file, `src/main/cpp/discord_jni.cpp` in the same folder, and Gradle only builds it when the
   `.aar` is there.
5. Build as usual with `npm run tauri android build -- --apk`, or run `npm run tauri android dev`.

The settings of the app then show **Discord Rich Presence** on Android. A build that has the id but
no SDK shows that the SDK is missing.

### How it fits together

```text
webview  ──discord__update──▶  src/api/discord.rs  ──Card::new──▶  Session::native("discord")
                                                                         │
                                  DiscordHost.kt ◀── SessionPlugin.kt ◀──┘
                                        │  reflection, so the build compiles without the SDK
                                        ▼
                                  discord_jni.cpp ──▶ Discord Social SDK ──▶ Discord app of the phone
```

- `DiscordHost` loads the library `yamp_discord`, which exists only in a build that has the SDK.
  That is how it tells whether the SDK is there.
- The SDK wants the activity before anything else, so `MainActivity` hands it over with
  `DiscordSocialSdkInit.setEngineActivity`. This is by reflection as well.
- `discord_jni.cpp` runs one thread that pumps the callbacks of the SDK and sends the last card
  that was asked for. A burst of updates, such as the one of a seek, costs one call.
- The card is cleared when the activity is destroyed. Only the webview knows what plays, and it
  goes with the activity, so the card would go stale. Opening the app shows it again.

### Size of the app

The SDK carries the libraries for voice chat, which yamp does not use, and they add to the size of
the apk. Discord describes in the voice chat guide of the Social SDK how to leave out the noise
cancellation part, which lowers the install size. A build only has to do it when the size matters.

### When nothing shows up on Android

- The Discord app has to be installed and signed in, and its setting for sharing the current
  activity has to be on.
- `available` false means the build has no application id or no SDK. Check that `config.toml` is set,
  that `libs/discord_partner_sdk.aar` is in place and that the build was started again after adding it.
- `adb logcat -s yamp-discord` shows when Discord refused a presence.
- A progress bar that is off by a factor of a thousand means the SDK reads another unit of time than
  the card sends. The card sends milliseconds, see `startMs` and `endMs` in `DiscordHost.kt`.

## The card

| Part       | Shows                                                                         |
| ---------- | ----------------------------------------------------------------------------- |
| Name       | the artist, which is what the status of the user reads after "Listening to"   |
| Details    | the title, a link when `url` is set                                           |
| State      | the artist, a link when `artist_url` is set                                   |
| Cover      | the large image when it is a web address, linked to `url`, and the hover text "Listening on yamp" |
| Timestamps | a progress bar when `end_ms` is set, otherwise a time that counts up          |
| Buttons    | "Listen on YouTube Music" when `url` is set, and "Get yamp", which leads to the repository |

The name of the application would make the status read "Listening to yamp", but the person
listens on yamp to something, so the activity overrides the name with the artist. The cover hover
text and the button are where yamp is named. Only addresses of a web page are linked, a local file
or a live station has none, and then the card has only the "Get yamp" button.

Discord refuses a line of less than 2 characters or of more than 128, so shorter lines are padded
with an invisible character and longer ones are cut. A cover that is a local file is left out,
Discord has no way to load it.

## When nothing shows up

- Discord has to run on the same machine, and its own setting for sharing the current activity
  has to be on.
- A sandboxed Discord, such as the Flatpak or Snap build, can keep its socket where other apps do
  not look for it. Try the regular install to rule that out.
- `available()` returning false means the build has no id, check `config.toml` and rebuild.
