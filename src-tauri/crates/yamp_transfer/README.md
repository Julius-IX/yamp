# yamp-transfer

Moves a library between Yamp and other music services through CSV files. It has no
dependencies, so `cargo test` here builds only this crate. The app reaches it through the
`transfer` command group in `src/api/transfer.rs`.

```rust
let parsed = yamp_transfer::parse(&text, "Road Trip")?;
for entry in &parsed.entries { /* look it up, then add it to the library */ }
```

The name given to `parse` is the file name without its extension. Tools like Exportify write
one file per playlist and put the playlist name only there.
