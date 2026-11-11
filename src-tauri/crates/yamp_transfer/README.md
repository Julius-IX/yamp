# yamp-transfer

Moves a library between Yamp and other music services through files. It reads CSV files and
the JSON of the account data that Spotify offers, and writes CSV. It depends on nothing but
`serde_json`, so `cargo test` here builds almost nothing else. The app reaches it through the
`transfer` command group in `src/api/transfer.rs`.

```rust
let parsed = yamp_transfer::parse(&text, "Road Trip")?;
for entry in &parsed.entries { /* look it up, then add it to the library */ }
```

The name given to `parse` is the file name without its extension. Tools like Exportify write
one file per playlist and put the playlist name only there.

## What can be read

- Spotify account data (`YourLibrary.json` for liked songs and saved albums, `Playlist1.json`
  and its siblings for playlists). The streaming history is refused, it is not a library.
- CSV from Exportify, SoundCloud tools and similar. Columns are found by name.
- CSV that Yamp wrote itself.
