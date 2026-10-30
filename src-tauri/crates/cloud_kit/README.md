# cloud-kit

Building blocks on top of `cloud-sync` for sharing data between devices. There are no schemas here, only the shapes to build them from. Everything works on any `CloudStore`.

## Dir

A directory is a key prefix, so nothing is created or removed. A `Dir` lists files and subdirectories and holds JSON documents with create, read, update and delete.

```rust
let root = Dir::root(store); // Arc<dyn CloudStore>
let me = root.dir(&format!("devices/{device_id}"))?;

me.put_json("settings.json", &settings)?;      // create or replace
me.insert_json("pairing.json", &pairing)?;     // fails if it exists
let s: Settings = me.get_json("settings.json")?;
me.update_json("settings.json", |mut s: Settings| { s.volume = 3; s })?;
me.delete("settings.json")?;

let devices = root.dir("devices")?.list_dirs()?; // ["abc", "def"]
```

## Log

An append only sequence of entries ordered by time. Each entry is a file named with a sortable timestamp (`2026-10-30T11-49-12.345Z.json`), so a reader keeps the last name it handled as a cursor.

```rust
let log = Log::new(root.dir(&format!("devices/{device_id}/log"))?);
log.append(&batch)?;

let entries = log.read_after::<Batch>(cursor.as_deref(), Some(100))?;
if let Some((last, _)) = entries.last() { /* store `last` as the new cursor */ }

log.trim_through(&acked_cursor)?; // drop entries every device has applied
```

Only one device appends to a log, so each device gets its own.

## Blobs

Big objects such as database snapshots. They are cut into chunks, written under a new generation and made visible by writing the manifest last.
Reads verify size and SHA-256. Both directions stream, so a snapshot never has to fit in memory.

```rust
let blobs = Blobs::new(root.dir("snapshots")?);
blobs.put_reader(&stamp::now(), std::fs::File::open("snapshot.db")?)?;

let latest = blobs.list_names()?.pop();
let mut tmp = std::fs::File::create("incoming.db")?;
blobs.get_to_writer(&latest.unwrap(), &mut tmp)?; // rename into place after it returns Ok
```

## Rules

- One writer per name. No backend has an atomic create, so `insert_*` checks and writes in two requests.
- Listings read everything below the directory. Keep trees shallow and give big groups of files a directory of their own.
- All calls block, run them on a blocking thread.
