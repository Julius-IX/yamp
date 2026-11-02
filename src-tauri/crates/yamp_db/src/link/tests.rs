//! Several devices around one cloud folder, kept in a temporary directory.

use std::{path::PathBuf, sync::Arc, thread::sleep, time::Duration};

use cloud_sync::{fs::LocalDirStore, CloudStore};

use super::*;
use crate::store::track::tests::track;

struct Rig {
  dir: PathBuf,
  store: Arc<dyn CloudStore>,
}

impl Rig {
  fn new(name: &str) -> Self {
    let dir = std::env::temp_dir().join(format!("yamp-db-link-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let store: Arc<dyn CloudStore> = Arc::new(LocalDirStore::new(&dir).unwrap());
    Self { dir, store }
  }

  /// A new device with an empty library.
  async fn device(&self, name: &str) -> (Db, Link) {
    let db = Db::memory().await.unwrap();
    let link = Link::open(&db, self.store.clone(), name).await.unwrap();
    (db, link)
  }
}

impl Drop for Rig {
  fn drop(&mut self) {
    let _ = std::fs::remove_dir_all(&self.dir);
  }
}

fn nap() {
  // changes made in the same millisecond would tie, and a tie goes by device id
  sleep(Duration::from_millis(3));
}

async fn names(db: &Db) -> Vec<(String, Vec<String>)> {
  let mut out = Vec::new();
  for p in db.playlists().list().await.unwrap() {
    let tracks = db.playlists().tracks(&p.id).await.unwrap();
    out.push((p.name, tracks.into_iter().map(|t| t.id).collect()));
  }
  out.sort();
  out
}

#[tokio::test]
async fn two_devices_converge_without_echo() {
  let rig = Rig::new("converge");
  let (a, la) = rig.device("Phone").await;
  let (b, lb) = rig.device("Laptop").await;

  a.likes().like(&track("t1", "One")).await.unwrap();
  let p = a.playlists().create("Mix", None).await.unwrap();
  a.playlists()
    .add_tracks(&p.id, &[track("t1", "One"), track("t2", "Two")])
    .await
    .unwrap();
  let first = la.sync().await.unwrap();
  assert_eq!(first.pushed, 4);

  let got = lb.sync().await.unwrap();
  assert_eq!((got.pulled, got.pushed), (4, 0));
  assert_eq!(b.likes().ids().await.unwrap(), ["t1"]);
  assert_eq!(names(&b).await, names(&a).await);

  // the laptop edits, the phone receives and sends nothing back
  nap();
  b.playlists()
    .add_tracks(&p.id, &[track("t3", "Three")])
    .await
    .unwrap();
  lb.sync().await.unwrap();
  let back = la.sync().await.unwrap();
  assert_eq!((back.pulled, back.pushed), (2, 0));
  assert_eq!(names(&a).await, names(&b).await);
  assert_eq!(names(&a).await[0].1, ["t1", "t2", "t3"]);
}

#[tokio::test]
async fn a_conflict_ends_the_same_on_both_devices() {
  let rig = Rig::new("conflict");
  let (a, la) = rig.device("Phone").await;
  let (b, lb) = rig.device("Laptop").await;
  let p = a.playlists().create("Start", None).await.unwrap();
  la.sync().await.unwrap();
  lb.sync().await.unwrap();

  // both rename it while offline, the laptop last
  a.playlists()
    .update(&p.id, "From phone", None)
    .await
    .unwrap();
  nap();
  b.playlists()
    .update(&p.id, "From laptop", None)
    .await
    .unwrap();
  for link in [&la, &lb, &la, &lb] {
    link.sync().await.unwrap();
  }

  assert_eq!(names(&a).await, names(&b).await);
  assert_eq!(names(&a).await[0].0, "From laptop");
}

#[tokio::test]
async fn deletions_travel_too() {
  let rig = Rig::new("delete");
  let (a, la) = rig.device("Phone").await;
  let (b, lb) = rig.device("Laptop").await;
  let p = a.playlists().create("Gone soon", None).await.unwrap();
  a.likes().like(&track("t1", "One")).await.unwrap();
  la.sync().await.unwrap();
  lb.sync().await.unwrap();
  assert_eq!(b.playlists().list().await.unwrap().len(), 1);

  nap();
  a.playlists().delete(&p.id).await.unwrap();
  a.likes().unlike("t1").await.unwrap();
  la.sync().await.unwrap();
  lb.sync().await.unwrap();

  assert!(b.playlists().list().await.unwrap().is_empty());
  assert!(b.likes().ids().await.unwrap().is_empty());
}

#[tokio::test]
async fn a_damaged_log_entry_is_skipped() {
  let rig = Rig::new("damaged");
  let (a, la) = rig.device("Phone").await;
  let (b, lb) = rig.device("Laptop").await;
  a.likes().like(&track("t1", "One")).await.unwrap();
  la.sync().await.unwrap();

  // a broken entry after the good one, then a good one after it
  let log_dir = rig.store_dir(la.device_id());
  std::fs::write(
    log_dir.join(format!("{}.json", cloud_kit::stamp::now())),
    b"{nope",
  )
  .unwrap();
  nap();
  a.likes().like(&track("t2", "Two")).await.unwrap();
  la.sync().await.unwrap();

  lb.sync().await.unwrap();
  assert_eq!(b.likes().count().await.unwrap(), 2);
}

#[tokio::test]
async fn devices_are_listed_and_can_be_forgotten() {
  let rig = Rig::new("devices");
  let (_a, la) = rig.device("Phone").await;
  let (_b, lb) = rig.device("Laptop").await;
  la.sync().await.unwrap();
  lb.sync().await.unwrap();

  let list = la.devices().await.unwrap();
  let seen: Vec<(&str, bool)> = list
    .iter()
    .map(|d| (d.name.as_str(), d.this_device))
    .collect();
  assert_eq!(seen, [("Laptop", false), ("Phone", true)]);

  assert!(la.forget_device(la.device_id()).await.is_err());
  la.forget_device(lb.device_id()).await.unwrap();
  assert_eq!(la.devices().await.unwrap().len(), 1);
}

impl Rig {
  /// The folder of a device's log on disk, for tests that damage it.
  fn store_dir(&self, device: &str) -> PathBuf {
    self.dir.join("devices").join(device).join("log")
  }
}
