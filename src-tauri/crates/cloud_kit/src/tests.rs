use std::{fs, path::PathBuf, sync::Arc};

use cloud_sync::{fs::LocalDirStore, CloudStore};
use serde::{Deserialize, Serialize};

use crate::{entry_time_ms, Blobs, Dir, Error, Log};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Op {
  kind: String,
  n: u32,
}

fn op(n: u32) -> Op {
  Op {
    kind: "add".into(),
    n,
  }
}

fn root(name: &str) -> (Dir, PathBuf) {
  let path = std::env::temp_dir().join(format!("cloud-kit-test-{}-{name}", std::process::id()));
  let _ = fs::remove_dir_all(&path);
  let store: Arc<dyn CloudStore> = Arc::new(LocalDirStore::new(&path).unwrap());
  (Dir::root(store), path)
}

#[test]
fn dir_lists_files_and_subdirs() {
  let (root, path) = root("dir");
  let devices = root.dir("devices").unwrap();
  devices.dir("a").unwrap().put_bytes("x.json", b"1").unwrap();
  devices.dir("a").unwrap().put_bytes("y.json", b"2").unwrap();
  devices.dir("b").unwrap().put_bytes("z.json", b"3").unwrap();
  devices.put_bytes("note", b"n").unwrap();

  assert_eq!(devices.list_dirs().unwrap(), ["a", "b"]);
  let names = |v: Vec<cloud_sync::ObjectMeta>| v.into_iter().map(|m| m.key).collect::<Vec<_>>();
  assert_eq!(names(devices.list().unwrap()), ["note"]);
  assert_eq!(
    names(devices.list_all().unwrap()),
    ["a/x.json", "a/y.json", "b/z.json", "note"]
  );
  assert_eq!(
    names(devices.dir("a").unwrap().list_prefix("x").unwrap()),
    ["x.json"]
  );
  assert!(devices.list_prefix("a/").is_err());

  assert_eq!(devices.dir("a").unwrap().delete_dir().unwrap(), 2);
  assert_eq!(devices.list_dirs().unwrap(), ["b"]);
  assert!(root.delete_dir().is_err());
  let _ = fs::remove_dir_all(path);
}

#[test]
fn json_crud() {
  let (root, path) = root("json");
  assert_eq!(root.get_json_opt::<Op>("a.json").unwrap(), None);
  root.insert_json("a.json", &op(1)).unwrap();
  assert!(matches!(
    root.insert_json("a.json", &op(2)),
    Err(Error::AlreadyExists(_))
  ));
  assert_eq!(root.get_json::<Op>("a.json").unwrap(), op(1));

  let updated = root
    .update_json("a.json", |mut o: Op| {
      o.n += 10;
      o
    })
    .unwrap();
  assert_eq!(updated.n, 11);
  root.put_json("a.json", &op(5)).unwrap();
  assert_eq!(root.get_json::<Op>("a.json").unwrap().n, 5);

  root.delete("a.json").unwrap();
  root.delete("a.json").unwrap();
  assert!(root.get_json::<Op>("a.json").unwrap_err().is_not_found());
  assert!(root
    .update_json("a.json", |o: Op| o)
    .unwrap_err()
    .is_not_found());
  let _ = fs::remove_dir_all(path);
}

#[test]
fn log_cursor_and_trim() {
  let (root, path) = root("log");
  let log = Log::new(root.dir("devices/a").unwrap());
  assert_eq!(log.latest().unwrap(), None);
  let names: Vec<String> = (0..5).map(|n| log.append(&op(n)).unwrap()).collect();
  assert!(names.windows(2).all(|w| w[0] < w[1]));
  assert!(entry_time_ms(&names[0]).is_some());
  root
    .dir("devices/a")
    .unwrap()
    .put_bytes("stray.txt", b"x")
    .unwrap();

  assert_eq!(log.names_after(None, None).unwrap(), names);
  assert_eq!(log.latest().unwrap().as_deref(), Some(names[4].as_str()));
  let page = log.read_after::<Op>(Some(&names[1]), Some(2)).unwrap();
  assert_eq!(
    page,
    vec![(names[2].clone(), op(2)), (names[3].clone(), op(3))]
  );

  assert_eq!(log.trim_through(&names[2]).unwrap(), 3);
  assert_eq!(log.names_after(None, None).unwrap(), names[3..]);
  assert!(root.dir("devices/a").unwrap().exists("stray.txt").unwrap());
  let _ = fs::remove_dir_all(path);
}

fn data(len: usize) -> Vec<u8> {
  (0..len).map(|i| (i * 7 % 251) as u8).collect()
}

#[test]
fn blob_roundtrip_in_chunks() {
  let (root, path) = root("blob");
  let blobs = Blobs::new(root.dir("snapshots").unwrap()).with_chunk_size(10);
  let bytes = data(95);

  let info = blobs.put("s1", &bytes).unwrap();
  assert_eq!((info.size, info.chunks), (95, 10));
  assert_eq!(blobs.get("s1").unwrap(), bytes);
  assert_eq!(blobs.info("s1").unwrap().unwrap(), info);

  // exact multiple of the chunk size and empty input
  assert_eq!(blobs.put("s2", &data(30)).unwrap().chunks, 3);
  assert_eq!(blobs.put("empty", &[]).unwrap().chunks, 0);
  assert!(blobs.get("empty").unwrap().is_empty());

  assert_eq!(blobs.list_names().unwrap(), ["empty", "s1", "s2"]);
  assert_eq!(blobs.list().unwrap().len(), 3);
  assert!(blobs.get("nope").unwrap_err().is_not_found());
  assert!(blobs.put("a/b", b"x").is_err());
  let _ = fs::remove_dir_all(path);
}

#[test]
fn blob_replace_removes_old_chunks() {
  let (root, path) = root("replace");
  let snaps = root.dir("snapshots").unwrap();
  let blobs = Blobs::new(snaps.clone()).with_chunk_size(10);
  blobs.put("s", &data(100)).unwrap();
  let new = data(25);
  let info = blobs.put("s", &new).unwrap();

  assert_eq!(blobs.get("s").unwrap(), new);
  // the manifest and the three chunks of the new generation, nothing from the old one
  assert_eq!(snaps.list_all().unwrap().len(), 1 + info.chunks as usize);

  assert!(matches!(
    blobs.insert("s", b"x"),
    Err(Error::AlreadyExists(_))
  ));
  blobs.delete("s").unwrap();
  blobs.delete("s").unwrap();
  assert!(snaps.list_all().unwrap().is_empty());
  blobs.insert("s", b"x").unwrap();
  let _ = fs::remove_dir_all(path);
}

#[test]
fn blob_detects_damage() {
  let (root, path) = root("damage");
  let snaps = root.dir("snapshots").unwrap();
  let blobs = Blobs::new(snaps.clone()).with_chunk_size(10);
  let info = blobs.put("s", &data(35)).unwrap();
  let chunks = snaps.dir("s").unwrap().dir(&info.generation).unwrap();

  chunks.put_bytes("00000001", b"0123456789").unwrap();
  assert!(matches!(blobs.get("s"), Err(Error::Corrupt(_))));

  chunks.delete("00000002").unwrap();
  assert!(matches!(blobs.get("s"), Err(Error::Corrupt(_))));
  let _ = fs::remove_dir_all(path);
}

#[test]
fn blob_streams_from_a_reader() {
  let (root, path) = root("stream");
  let blobs = Blobs::new(root.dir("snapshots").unwrap()).with_chunk_size(16);
  let bytes = data(100);
  blobs
    .put_reader("s", std::io::Cursor::new(bytes.clone()))
    .unwrap();
  let mut out = Vec::new();
  let info = blobs.get_to_writer("s", &mut out).unwrap();
  assert_eq!((out, info.size), (bytes, 100));
  let _ = fs::remove_dir_all(path);
}
