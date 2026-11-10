# yamp-discord

Discord Rich Presence, the "listening to" card on the Discord profile of the user. It has no Tauri
dependency, so `cargo test` here builds only this crate. The app reaches it through the `discord`
command group in `src/api/discord.rs`.

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

Only a desktop can talk to Discord, over a local socket. On Android and iOS every call does
nothing, and the dependency on `discord-rich-presence` is not compiled in at all.

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
