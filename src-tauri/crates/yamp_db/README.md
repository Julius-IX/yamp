# yamp-db

SQLite storage for the music library, built on `sqlx`. It owns the schema and every query, and links the library of several devices through `cloud-kit`.
There is no Tauri dependency, so the crate can be checked and tested alone.

## Usage

```rust
let db = Db::open(app_data.join("library.db")).await?;

db.likes().like(&track).await?;
let playlist = db.playlists().create("Road trip", None).await?;
db.playlists().add_tracks(&playlist.id, &[track]).await?;
```

A store takes the `Track` it is given and saves its metadata in the same transaction. A like, a playlist entry or a download therefore never points at a track the library cannot show.

| Store | Holds |
| ----- | ----- |
| `tracks()` | Track metadata, lookup and search |
| `likes()` | Liked tracks, newest first |
| `artists()` | Followed artists |
| `playlists()` | Playlists, with reordering |
| `downloads()` | Records of downloaded audio |
| `locals()` | Folders with local audio, the tracks found in them, and the downloads and those tracks listed as one |

Timestamps are milliseconds since the Unix epoch.

## Schema

The schema is the list of files in `src-tauri/migrations`, applied in order by `sqlx::migrate!` and recorded with a checksum in `_sqlx_migrations`.
A file is never edited after a release, a change is a new file at the end. A database written by a newer app is refused.

Every query is checked at compile time by the `sqlx` macros against a database built from those migrations.
A normal build has no such database and reads the answers from `src-tauri/.sqlx`, which is committed. SQLite is bundled, which gives every platform the same version.

## Changing a query or the schema

The checks need a database and `sqlx-cli` (`cargo install sqlx-cli --no-default-features --features sqlite`). From `src-tauri`:

```sh
export DATABASE_URL=sqlite:target/dev.db
sqlx database setup          # creates the file and applies the migrations
cargo check -p yamp-db       # checks the queries against it
cargo sqlx prepare --workspace   # rewrites .sqlx, commit the result
```

With `DATABASE_URL` set a build checks against that database, without it the committed data is used.
`SQLX_OFFLINE=true` forces the committed data, which is what CI should do. A stale `.sqlx` shows up as a build error about a missing query.

Tables hold track ids without foreign keys to `tracks`. A change from another device may reference a track that arrives later, and lists skip such entries until it does.

## Downloads

`MediaStore` ties the `downloads` records to a folder of audio files. A file is named `track_id.ext`, so the primary key is the file name and the record is the only index.
Ids are limited to letters, digits, `-` and `_` for that reason.

`import` moves a finished download into the folder. It lands under a hidden `.part` name and is renamed into place, so a reader never sees a half written file.
`repair` drops records whose file is gone and deletes files no record names. Run it at app start, not while an import is running.

Downloads belong to one device and are never linked.

## Local files

`locals()` keeps a list of folders. A scan turns every audio file below them into a track, with the tags as metadata and the file name as a fallback. The track sits in `tracks` like a streamed one, so likes, playlists and the queue treat it the same. Its id is `local-` and a hash of the path, which gives the same file the same track on every scan.

A scan reads the tags of a file only when its size or modification time changed. A folder that cannot be read keeps its tracks, since an unplugged drive looks like an empty folder.

The ids with the `local-` prefix are not linked, see `0010_local_files.sql`. A like or a playlist entry of such a track is, and a device without the file skips it until it has one.

## Linking devices

`Link` converges the library of every device through a private folder on the user's cloud account. It takes any `cloud_sync::CloudStore`, so the provider and its login stay with the app.

```rust
let link = Link::open(&db, store, "Pixel 9").await?;
link.sync().await?;     // receive, send, report progress
link.compact().await?;  // now and then, for example weekly
```

### What is shared

Synced data:
- Tracks, except local ones
- Likes
- Followed artists
- Playlists

An entity travels as one document, a playlist with its whole track list. Triggers in `0007_link.sql` note every change in `link_dirty`, so no store has to know about syncing.

Two devices that change the same entity meanwhile do not merge. The change with the later time wins and equal times go to the greater device id, so every device picks the same one.
A device whose clock is behind still wins over an older version it had seen, because it sends its change with a time after that version. Large clock errors can still make an older edit win.

A deletion is a change too. Its tombstone is kept so that an old copy arriving late does not bring the entity back.

### Folder layout

```text
devices/<id>/meta.json      name, last seen, which entries of the others were read
devices/<id>/log/…          what this device changed, one entry per push
devices/<id>/snapshot.json  what the snapshot of this device covers
snapshots/<id>              the whole library of this device, as a blob
```

Every file has one writer, the device it is named after, which is the rule `cloud-kit` asks for since no backend has atomic writes.
A device sends by appending to its own log and receives by reading the logs of the others from a stored cursor. The cursor moves in the same transaction as the changes it covers.

### Cleanup and snapshots

`compact` writes a snapshot and deletes log entries that every device active in the last 60 days has read. A device that has not reported at all blocks the cleanup.
The device records `trimmed_through` in its meta file before it deletes anything.

A new device, or one that was away longer, finds that a log has a gap for it and starts from the newest usable snapshot.
It then reads each log from where the snapshot ends. With no usable snapshot `sync` fails with `Error::NoSnapshot`, and any other device running `compact` fixes that.

### Limits

- The device id is stored in the database. A copy of the file must not be used as a second device.
- Snapshots are built in memory. That is a few megabytes for tens of thousands of tracks.
- Tombstones are kept forever.
- All cloud calls block and run on a blocking thread.
