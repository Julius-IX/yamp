//! Downloaded audio on disk.
//!
//! The folder holds only audio files, each named `track_id.ext` after the primary key of
//! its record in `downloads`. The record is what the rest of the app reads, the folder is
//! never listed to find a track. Both can drift apart, for example when the OS clears
//! cached files or the app dies mid-import, and [`MediaStore::repair`] reconciles them.
//!
//! A file is first moved to a hidden `.part` name in the folder and then renamed into
//! place. The rename is atomic, so a reader sees either the old complete file or the new
//! one and never a half copied file.

use std::{
  collections::HashSet,
  io,
  path::{Path, PathBuf},
};

use crate::{store::download_name_check, Db, Download, Downloads, Error, Result, Track};

/// What [`MediaStore::repair`] fixed.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RepairReport {
  /// Records dropped because their file is gone.
  pub missing_files: Vec<String>,
  /// Files deleted because no record names them, including leftover partial imports.
  pub orphan_files: Vec<String>,
}

/// The download folder together with the records of what is in it.
#[derive(Clone)]
pub struct MediaStore {
  downloads: Downloads,
  root: PathBuf,
}

impl MediaStore {
  /// Uses `root` as the download folder and creates it when missing.
  pub async fn open(db: &Db, root: impl Into<PathBuf>) -> Result<Self> {
    let root = root.into();
    tokio::fs::create_dir_all(&root).await?;
    Ok(Self {
      downloads: db.downloads(),
      root,
    })
  }

  pub fn root(&self) -> &Path {
    &self.root
  }

  pub fn downloads(&self) -> &Downloads {
    &self.downloads
  }

  /// Where the audio of a track is, or `None` when it is not downloaded. A record whose
  /// file has vanished counts as not downloaded.
  pub async fn path(&self, track_id: &str) -> Result<Option<PathBuf>> {
    let Some(record) = self.downloads.get(track_id).await? else {
      return Ok(None);
    };
    let path = self.root.join(record.file_name());
    Ok(tokio::fs::try_exists(&path).await?.then_some(path))
  }

  /// Takes a finished download at `source` into the folder and records it. The format is
  /// the extension of `source`. The source file is moved, so it is gone afterwards.
  ///
  /// The file lands before the record is written. A failure in between leaves a file that
  /// no record names, which [`repair`](Self::repair) removes.
  pub async fn import(&self, track: &Track, source: &Path) -> Result<Download> {
    let ext = source
      .extension()
      .and_then(|e| e.to_str())
      .map(str::to_ascii_lowercase)
      .unwrap_or_default();
    download_name_check(&track.id, &ext)?;

    let name = format!("{}.{ext}", track.id);
    let part = self.root.join(format!(".{name}.part"));
    let dest = self.root.join(&name);
    move_file(source, &part).await?;
    let size = tokio::fs::metadata(&part).await?.len();
    if let Err(e) = tokio::fs::rename(&part, &dest).await {
      let _ = tokio::fs::remove_file(&part).await;
      return Err(e.into());
    }

    let previous = self.downloads.record(track, &ext, size).await?;
    if let Some(old) = previous.filter(|old| old.ext != ext) {
      // another format of the same track, the old file would be left behind otherwise
      remove_if_exists(&self.root.join(old.file_name())).await?;
    }
    Ok(Download {
      track_id: track.id.clone(),
      ext,
      size_bytes: size,
      downloaded_ms: self
        .downloads
        .get(&track.id)
        .await?
        .map_or(0, |d| d.downloaded_ms),
    })
  }

  /// Deletes the audio file and its record. Returns whether the track was downloaded.
  /// The file goes first, so a failure keeps the record and the delete can be retried.
  pub async fn remove(&self, track_id: &str) -> Result<bool> {
    let Some(record) = self.downloads.get(track_id).await? else {
      return Ok(false);
    };
    remove_if_exists(&self.root.join(record.file_name())).await?;
    self.downloads.remove(track_id).await?;
    Ok(true)
  }

  /// Makes records and files agree again. Meant for app start. It must not run while an
  /// import is in progress, since the `.part` file of that import counts as leftover.
  pub async fn repair(&self) -> Result<RepairReport> {
    let mut report = RepairReport::default();
    let records = self.downloads.list().await?;

    for record in &records {
      if !tokio::fs::try_exists(self.root.join(record.file_name())).await? {
        self.downloads.remove(&record.track_id).await?;
        report.missing_files.push(record.track_id.clone());
      }
    }

    let known: HashSet<String> = records.iter().map(Download::file_name).collect();
    let mut entries = tokio::fs::read_dir(&self.root).await?;
    while let Some(entry) = entries.next_entry().await? {
      if !entry.file_type().await?.is_file() {
        continue;
      }
      let Ok(name) = entry.file_name().into_string() else {
        continue;
      };
      if !known.contains(&name) {
        remove_if_exists(&entry.path()).await?;
        report.orphan_files.push(name);
      }
    }
    report.missing_files.sort();
    report.orphan_files.sort();
    Ok(report)
  }
}

/// A rename is instant on one file system. Between two it fails, and the file is copied
/// and the source removed instead, which is the usual case for the cache folder of a
/// downloader on Android and when the music folder is on another drive.
async fn move_file(from: &Path, to: &Path) -> Result<()> {
  if tokio::fs::rename(from, to).await.is_ok() {
    return Ok(());
  }
  if let Err(e) = tokio::fs::copy(from, to).await {
    let _ = tokio::fs::remove_file(to).await;
    return Err(e.into());
  }
  // the copy is complete, a source that cannot be removed is only wasted space
  if let Err(e) = tokio::fs::remove_file(from).await {
    log::warn!("could not remove {} after copying: {e}", from.display());
  }
  Ok(())
}

async fn remove_if_exists(path: &Path) -> Result<()> {
  match tokio::fs::remove_file(path).await {
    Ok(()) => Ok(()),
    Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
    Err(e) => Err(Error::Io(e)),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::store::track::tests::track;

  /// A fresh folder with an in-memory database and a scratch folder for fake downloads.
  async fn setup(name: &str) -> (MediaStore, PathBuf, PathBuf) {
    let base = std::env::temp_dir().join(format!("yamp-db-media-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let scratch = base.join("scratch");
    std::fs::create_dir_all(&scratch).unwrap();
    let db = Db::memory().await.unwrap();
    let store = MediaStore::open(&db, base.join("media")).await.unwrap();
    (store, scratch, base)
  }

  fn fake(scratch: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = scratch.join(name);
    std::fs::write(&path, bytes).unwrap();
    path
  }

  #[tokio::test]
  async fn import_moves_the_file_under_its_key() {
    let (store, scratch, base) = setup("import").await;
    let source = fake(&scratch, "download-1.M4A", b"audio");

    let d = store.import(&track("abc", "A"), &source).await.unwrap();
    assert_eq!((d.file_name().as_str(), d.size_bytes), ("abc.m4a", 5));
    assert!(!source.exists());
    let path = store.path("abc").await.unwrap().unwrap();
    assert_eq!(std::fs::read(path).unwrap(), b"audio");
    assert_eq!(store.downloads().total_size().await.unwrap(), 5);
    let _ = std::fs::remove_dir_all(base);
  }

  #[tokio::test]
  async fn a_new_format_replaces_the_old_file() {
    let (store, scratch, base) = setup("format").await;
    store
      .import(&track("abc", "A"), &fake(&scratch, "x.webm", b"old"))
      .await
      .unwrap();
    store
      .import(&track("abc", "A"), &fake(&scratch, "y.m4a", b"newer"))
      .await
      .unwrap();

    assert!(!store.root().join("abc.webm").exists());
    assert!(store.root().join("abc.m4a").exists());
    assert_eq!(store.downloads().count().await.unwrap(), 1);
    let _ = std::fs::remove_dir_all(base);
  }

  #[tokio::test]
  async fn import_refuses_what_cannot_be_named() {
    let (store, scratch, base) = setup("names").await;
    let no_ext = fake(&scratch, "noext", b"x");

    assert!(matches!(
      store.import(&track("abc", "A"), &no_ext).await,
      Err(Error::Invalid(_))
    ));
    assert!(matches!(
      store
        .import(&track("../evil", "A"), &fake(&scratch, "a.m4a", b"x"))
        .await,
      Err(Error::Invalid(_))
    ));
    // nothing was taken in
    assert!(no_ext.exists());
    assert_eq!(std::fs::read_dir(store.root()).unwrap().count(), 0);
    let _ = std::fs::remove_dir_all(base);
  }

  #[tokio::test]
  async fn remove_deletes_file_and_record() {
    let (store, scratch, base) = setup("remove").await;
    store
      .import(&track("abc", "A"), &fake(&scratch, "x.m4a", b"audio"))
      .await
      .unwrap();

    assert!(store.remove("abc").await.unwrap());
    assert!(!store.remove("abc").await.unwrap());
    assert!(!store.root().join("abc.m4a").exists());
    assert_eq!(store.path("abc").await.unwrap(), None);
    let _ = std::fs::remove_dir_all(base);
  }

  #[tokio::test]
  async fn repair_reconciles_records_and_files() {
    let (store, scratch, base) = setup("repair").await;
    for id in ["keep", "lost"] {
      store
        .import(&track(id, id), &fake(&scratch, "x.m4a", b"audio"))
        .await
        .unwrap();
    }
    std::fs::remove_file(store.root().join("lost.m4a")).unwrap();
    std::fs::write(store.root().join("stray.m4a"), b"x").unwrap();
    std::fs::write(store.root().join(".half.m4a.part"), b"x").unwrap();

    let report = store.repair().await.unwrap();
    assert_eq!(report.missing_files, ["lost"]);
    assert_eq!(report.orphan_files, [".half.m4a.part", "stray.m4a"]);
    assert_eq!(store.downloads().count().await.unwrap(), 1);
    assert!(store.root().join("keep.m4a").exists());
    assert_eq!(store.repair().await.unwrap(), RepairReport::default());
    let _ = std::fs::remove_dir_all(base);
  }
}
