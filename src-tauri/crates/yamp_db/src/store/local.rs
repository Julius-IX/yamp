//! Audio that is read from folders on this device.
//!
//! The user adds folders and a scan turns every audio file below them into a track. The track
//! lives in `tracks` like a streamed one, which is what lets likes, playlists and the queue treat
//! it the same. Only its id gives it away, see [`LOCAL_PREFIX`]. The path of the file lives in
//! `local_files` and is never shared with other devices, see `link`.
//!
//! Together with the downloads, these tracks make up what the app calls the local library. The
//! two are listed and counted here as one, while [`MediaStore`](crate::MediaStore) owns the files
//! of the downloads.

use std::{
  collections::{HashMap, HashSet},
  io,
  path::{Path, PathBuf},
  time::UNIX_EPOCH,
};

use lofty::{picture::PictureType, prelude::*, read_from_path};
use sqlx::{Row, SqlitePool, sqlite::SqliteRow};

use super::{
  cover,
  track::{self, Track},
};
use crate::{Db, Error, Result, clock::now_ms};

/// Ids of local tracks start with this. The sync skips them, see `0010_local_files`, so the
/// prefix must not change.
pub const LOCAL_PREFIX: &str = "local-";

pub fn is_local_id(id: &str) -> bool {
  id.starts_with(LOCAL_PREFIX)
}

/// Containers that a browser engine can play, since the audio element does the decoding.
const AUDIO_EXTS: &[&str] = &["mp3", "m4a", "aac", "flac", "ogg", "oga", "opus", "wav", "weba", "webm"];

const UNKNOWN_ARTIST: &str = "Unknown artist";

/// Thumbnails of local tracks are not URLs. They name a cover file that a scan cached, as
/// `cover:<name>`, and the app turns the name into a URL where it draws the cover. The file
/// lives on this device only, so a path would be of no use anywhere else.
pub const COVER_PREFIX: &str = "cover:";

/// Edge of a cached cover. The lists draw covers small and the largest place is the header of a
/// playlist, so this stays sharp there without keeping the full size of the embedded picture.
const LOCAL_COVER_SIZE: u32 = 320;

/// The id of the track for a file. It comes from the path alone, so a scan that finds the same
/// file again gets the same track and the likes and playlist entries of it still point at it.
/// A file that is moved or renamed is a new track. The hash is FNV-1a, since the standard
/// hasher is not guaranteed to give the same value in another build.
pub fn track_id_for(path: &Path) -> String {
  format!("{LOCAL_PREFIX}{:016x}", fnv1a(path.to_string_lossy().as_bytes()))
}

/// A number of songs and the bytes their files take.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Totals {
  pub songs: u64,
  pub bytes: u64,
}

/// What the local library holds, split by where the files came from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LocalUsage {
  /// Songs that were downloaded in the app.
  pub downloaded: Totals,
  /// Songs that a scan found in the added folders.
  pub folders: Totals,
}

/// What one [`Locals::scan`] changed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScanReport {
  /// Files that are new or whose tags were read again.
  pub added: usize,
  /// Files that are gone.
  pub removed: usize,
  /// The folder could not be read, so nothing was changed.
  pub unreachable: bool,
}

impl Db {
  pub fn locals(&self) -> Locals {
    Locals::new(self.pool().clone())
  }
}

/// A file found by a scan whose tags are not in the library yet or have changed.
struct Changed {
  path: String,
  size_bytes: u64,
  modified_ms: i64,
  track: Track,
}

/// The folders with local audio, the files found in them and the combined list of local songs.
#[derive(Clone)]
pub struct Locals {
  pool: SqlitePool,
}

impl Locals {
  pub(crate) fn new(pool: SqlitePool) -> Self {
    Self { pool }
  }

  pub async fn dirs(&self) -> Result<Vec<String>> {
    let rows = sqlx::query("SELECT path FROM local_dirs ORDER BY added_ms, path")
      .fetch_all(&self.pool)
      .await?;
    rows.iter().map(|r| Ok(r.try_get("path")?)).collect()
  }

  /// Returns whether the folder was new. A folder that lies inside another one, or around it, is
  /// refused. Both would find the same files, and a file can only belong to one folder.
  pub async fn add_dir(&self, dir: &Path) -> Result<bool> {
    let text = dir
      .to_str()
      .ok_or_else(|| Error::Invalid(format!("{dir:?} is not a usable folder name")))?;
    for known in self.dirs().await? {
      let known = Path::new(&known);
      if known == dir {
        return Ok(false);
      }
      if known.starts_with(dir) || dir.starts_with(known) {
        return Err(Error::Invalid(format!(
          "{text} overlaps {}, which is already added",
          known.display()
        )));
      }
    }
    sqlx::query("INSERT INTO local_dirs (path, added_ms) VALUES (?1, ?2)")
      .bind(text)
      .bind(now_ms())
      .execute(&self.pool)
      .await?;
    Ok(true)
  }

  /// Forgets the folder and every track that was found in it. Returns whether it was added.
  /// The files on disk are not touched. Likes and playlist entries of the tracks stay, and show
  /// up again when the folder is added back, since the ids come from the paths.
  pub async fn remove_dir(&self, dir: &str) -> Result<bool> {
    let mut tx = self.pool.begin().await?;
    sqlx::query("DELETE FROM tracks WHERE id IN (SELECT track_id FROM local_files WHERE dir = ?1)")
      .bind(dir)
      .execute(&mut *tx)
      .await?;
    sqlx::query("DELETE FROM local_files WHERE dir = ?1")
      .bind(dir)
      .execute(&mut *tx)
      .await?;
    let done = sqlx::query("DELETE FROM local_dirs WHERE path = ?1")
      .bind(dir)
      .execute(&mut *tx)
      .await?;
    tx.commit().await?;
    Ok(done.rows_affected() > 0)
  }

  /// Brings the tracks of a folder up to date. Only a file whose size or modification time
  /// differs from the last scan has its tags read, since that is the slow part.
  ///
  /// A folder that cannot be read leaves everything as it was. An unplugged drive or a network
  /// share that is offline looks the same as an emptied folder, and dropping the tracks for it
  /// would take their likes out of the lists until the next scan.
  ///
  /// Covers that the files carry are cached as JPEG files in `covers`.
  pub async fn scan(&self, dir: &str, covers: &Path) -> Result<ScanReport> {
    let known = self.known_files(dir).await?;
    let root = PathBuf::from(dir);
    let walked = {
      let known = known.clone();
      let covers = covers.to_path_buf();
      tokio::task::spawn_blocking(move || walk(&root, &known, &covers))
        .await
        .map_err(|e| Error::Io(io::Error::other(e)))?
    };
    let (seen, changed) = match walked {
      Ok(found) => found,
      Err(e) => {
        log::warn!("local folder {dir} could not be read: {e}");
        return Ok(ScanReport { unreachable: true, ..ScanReport::default() });
      }
    };

    let gone: Vec<&String> = known.keys().filter(|p| !seen.contains(*p)).collect();
    let mut tx = self.pool.begin().await?;
    let now = now_ms();
    for file in &changed {
      track::upsert(&mut tx, &file.track).await?;
      sqlx::query(
        "INSERT INTO local_files (track_id, path, dir, size_bytes, modified_ms, added_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT (track_id) DO UPDATE SET
           dir = excluded.dir, size_bytes = excluded.size_bytes, modified_ms = excluded.modified_ms",
      )
      .bind(&file.track.id)
      .bind(&file.path)
      .bind(dir)
      .bind(file.size_bytes as i64)
      .bind(file.modified_ms)
      .bind(now)
      .execute(&mut *tx)
      .await?;
    }
    for path in &gone {
      sqlx::query("DELETE FROM tracks WHERE id IN (SELECT track_id FROM local_files WHERE path = ?1)")
        .bind(path.as_str())
        .execute(&mut *tx)
        .await?;
      sqlx::query("DELETE FROM local_files WHERE path = ?1")
        .bind(path.as_str())
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(ScanReport { added: changed.len(), removed: gone.len(), unreachable: false })
  }

  /// Path to size and modification time, for the files that the last scan found in `dir`.
  async fn known_files(&self, dir: &str) -> Result<HashMap<String, (u64, i64)>> {
    let rows = sqlx::query("SELECT path, size_bytes, modified_ms FROM local_files WHERE dir = ?1")
      .bind(dir)
      .fetch_all(&self.pool)
      .await?;
    rows
      .iter()
      .map(|r| {
        let size: i64 = r.try_get("size_bytes")?;
        Ok((r.try_get("path")?, (size as u64, r.try_get("modified_ms")?)))
      })
      .collect()
  }

  /// Where the file of a local track is. `None` for any other track, and for a file that is gone.
  pub async fn path(&self, track_id: &str) -> Result<Option<PathBuf>> {
    let found: Option<String> = sqlx::query_scalar("SELECT path FROM local_files WHERE track_id = ?1")
      .bind(track_id)
      .fetch_optional(&self.pool)
      .await?;
    let Some(path) = found.map(PathBuf::from) else {
      return Ok(None);
    };
    Ok(tokio::fs::try_exists(&path).await?.then_some(path))
  }

  /// Deletes cached covers that no track names anymore. A cover is shared by every track of an
  /// album, so one is only unused once the last of them is gone.
  pub async fn prune_covers(&self, covers: &Path) -> Result<usize> {
    let rows: Vec<String> = sqlx::query_scalar("SELECT DISTINCT thumbnail FROM tracks WHERE thumbnail LIKE 'cover:%'")
      .fetch_all(&self.pool)
      .await?;
    let used: HashSet<String> = rows.iter().filter_map(|t| t.strip_prefix(COVER_PREFIX)).map(|n| format!("{n}.jpg")).collect();
    let covers = covers.to_path_buf();
    tokio::task::spawn_blocking(move || {
      let Ok(entries) = std::fs::read_dir(&covers) else { return 0 };
      let mut removed = 0;
      for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !used.contains(&name) && std::fs::remove_file(entry.path()).is_ok() {
          removed += 1;
        }
      }
      removed
    })
    .await
    .map_err(|e| Error::Io(io::Error::other(e)))
  }

  /// Local songs, newest first. These are the files of the added folders and the downloads. A
  /// song whose metadata is missing is skipped.
  pub async fn tracks(&self, limit: u32, offset: u32) -> Result<Vec<Track>> {
    let rows = sqlx::query(
      "SELECT t.id, t.title, t.artist, t.album, t.duration_secs, t.thumbnail, t.release_year
       FROM tracks t
       JOIN (SELECT track_id, added_ms AS at FROM local_files
             UNION ALL
             SELECT track_id, downloaded_ms FROM downloads) l ON l.track_id = t.id
       ORDER BY l.at DESC, t.id LIMIT ?1 OFFSET ?2",
    )
    .bind(i64::from(limit))
    .bind(i64::from(offset))
    .fetch_all(&self.pool)
    .await?;
    rows.iter().map(track_from_row).collect()
  }

  /// Counts what [`tracks`](Self::tracks) can return.
  pub async fn count(&self) -> Result<u64> {
    let n: i64 = sqlx::query_scalar(
      "SELECT (SELECT COUNT(*) FROM local_files f JOIN tracks t ON t.id = f.track_id)
            + (SELECT COUNT(*) FROM downloads d JOIN tracks t ON t.id = d.track_id)",
    )
    .fetch_one(&self.pool)
    .await?;
    Ok(n as u64)
  }

  pub async fn usage(&self) -> Result<LocalUsage> {
    Ok(LocalUsage {
      downloaded: self.totals("downloads").await?,
      folders: self.totals("local_files").await?,
    })
  }

  async fn totals(&self, table: &str) -> Result<Totals> {
    // the table name is one of two literals of this file and never comes from outside
    let row = sqlx::query(&format!("SELECT COUNT(*), COALESCE(SUM(size_bytes), 0) FROM {table}"))
      .fetch_one(&self.pool)
      .await?;
    let (songs, bytes): (i64, i64) = (row.try_get(0)?, row.try_get(1)?);
    Ok(Totals { songs: songs as u64, bytes: bytes as u64 })
  }
}

fn track_from_row(row: &SqliteRow) -> Result<Track> {
  let year: Option<i64> = row.try_get("release_year")?;
  Ok(Track {
    id: row.try_get("id")?,
    title: row.try_get("title")?,
    artist: row.try_get("artist")?,
    album: row.try_get("album")?,
    duration_secs: row.try_get("duration_secs")?,
    thumbnail: row.try_get("thumbnail")?,
    release_year: year.and_then(|y| u32::try_from(y).ok()),
  })
}

/// Lists the audio files below `root`. Returns every path that was found, and the files among
/// them that `known` does not already hold in the same size and modification time, with their
/// tags read. Fails only when `root` itself cannot be read, a folder below it that is locked is
/// skipped. Links are not followed, which rules out loops.
fn walk(root: &Path, known: &HashMap<String, (u64, i64)>, covers: &Path) -> io::Result<(HashSet<String>, Vec<Changed>)> {
  let (mut seen, mut changed) = (HashSet::new(), Vec::new());
  let mut pending = vec![root.to_path_buf()];
  let mut at_root = true;
  while let Some(dir) = pending.pop() {
    let entries = match std::fs::read_dir(&dir) {
      Ok(entries) => entries,
      Err(e) if at_root => return Err(e),
      Err(e) => {
        log::warn!("skipped {}: {e}", dir.display());
        continue;
      }
    };
    at_root = false;
    for entry in entries.flatten() {
      // hidden files are mostly leftovers, like the `._` copies that macOS writes on other drives
      if entry.file_name().to_string_lossy().starts_with('.') {
        continue;
      }
      let (Ok(kind), path) = (entry.file_type(), entry.path()) else { continue };
      if kind.is_dir() {
        pending.push(path);
        continue;
      }
      if !kind.is_file() || !is_audio(&path) {
        continue;
      }
      let (Some(key), Ok(meta)) = (path.to_str().map(str::to_owned), entry.metadata()) else { continue };
      let modified_ms = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_millis() as i64);
      let stamp = (meta.len(), modified_ms);
      if known.get(&key) != Some(&stamp) {
        let track = read_track(&path, covers);
        changed.push(Changed { path: key.clone(), size_bytes: stamp.0, modified_ms, track });
      }
      seen.insert(key);
    }
  }
  Ok((seen, changed))
}

fn is_audio(path: &Path) -> bool {
  path
    .extension()
    .and_then(|e| e.to_str())
    .is_some_and(|e| AUDIO_EXTS.contains(&e.to_ascii_lowercase().as_str()))
}

/// The track for a file. A file without readable tags still plays, it is named after the file.
fn read_track(path: &Path, covers: &Path) -> Track {
  let mut track = Track {
    id: track_id_for(path),
    title: path
      .file_stem()
      .map_or_else(|| "Unknown".into(), |s| s.to_string_lossy().into_owned()),
    artist: UNKNOWN_ARTIST.into(),
    album: None,
    duration_secs: None,
    thumbnail: None,
    release_year: None,
  };
  let tagged = match read_from_path(path) {
    Ok(tagged) => tagged,
    Err(e) => {
      log::debug!("no tags in {}: {e}", path.display());
      return track;
    }
  };
  let seconds = tagged.properties().duration().as_secs_f64();
  track.duration_secs = (seconds > 0.0).then_some(seconds);
  if let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) {
    let text = |value: Option<std::borrow::Cow<'_, str>>| {
      value.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty())
    };
    track.title = text(tag.title()).unwrap_or(track.title);
    track.artist = text(tag.artist()).unwrap_or(track.artist);
    track.album = text(tag.album());
    track.release_year = tag.year();
    let picture = tag
      .pictures()
      .iter()
      .find(|p| p.pic_type() == PictureType::CoverFront)
      .or_else(|| tag.pictures().first());
    if let Some(name) = picture.and_then(|p| cache_cover(p.data(), covers)) {
      track.thumbnail = Some(format!("{COVER_PREFIX}{name}"));
    }
  }
  track
}

/// Writes the picture of a file into `covers` and returns the name it got, without extension.
/// The name is a hash of the picture, so the tracks of an album that all carry the same one share a
/// file and a picture that is cached already is not decoded again. A picture that cannot be read
/// is skipped, the track then shows the placeholder like one without any.
fn cache_cover(picture: &[u8], covers: &Path) -> Option<String> {
  let name = format!("{:016x}", fnv1a(picture));
  let target = covers.join(format!("{name}.jpg"));
  if target.exists() {
    return Some(name);
  }
  let jpeg = match cover::scale_to(picture, LOCAL_COVER_SIZE) {
    Ok(jpeg) => jpeg,
    Err(e) => {
      log::debug!("a cover could not be used: {e}");
      return None;
    }
  };
  // renamed into place, so a reader never meets a half written file
  let part = covers.join(format!(".{name}.part"));
  let written = std::fs::create_dir_all(covers)
    .and_then(|()| std::fs::write(&part, jpeg))
    .and_then(|()| std::fs::rename(&part, &target));
  if let Err(e) = written {
    log::warn!("could not cache a cover: {e}");
    let _ = std::fs::remove_file(&part);
    return None;
  }
  Some(name)
}

fn fnv1a(bytes: &[u8]) -> u64 {
  let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
  for byte in bytes {
    hash ^= u64::from(*byte);
    hash = hash.wrapping_mul(0x0100_0000_01b3);
  }
  hash
}

/// A folder below another, for choosing a folder where the system has no picker for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubDir {
  pub name: String,
  pub path: String,
}

/// The folders directly inside `path`, by name. Hidden ones and links are left out.
pub fn list_subdirs(path: &Path) -> io::Result<Vec<SubDir>> {
  let mut out: Vec<SubDir> = std::fs::read_dir(path)?
    .flatten()
    .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
    .filter_map(|e| {
      let name = e.file_name().into_string().ok()?;
      let path = e.path().into_os_string().into_string().ok()?;
      (!name.starts_with('.')).then_some(SubDir { name, path })
    })
    .collect();
  out.sort_by_key(|d| d.name.to_lowercase());
  Ok(out)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::store::track::tests::track;

  /// A scratch folder with a few fake audio files. Their bytes are not audio, which exercises the
  /// fallback to the file name.
  fn scratch(name: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!("yamp-db-local-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("sub")).unwrap();
    std::fs::write(base.join("one.mp3"), b"x").unwrap();
    std::fs::write(base.join("sub").join("two.FLAC"), b"xx").unwrap();
    std::fs::write(base.join("notes.txt"), b"x").unwrap();
    std::fs::write(base.join(".hidden.mp3"), b"x").unwrap();
    base
  }

  /// The folder for cached covers of a test, next to its scratch folder and not inside it.
  fn covers_of(base: &Path) -> PathBuf {
    base.with_file_name(format!("{}-covers", base.file_name().unwrap().to_string_lossy()))
  }

  async fn scanned(name: &str) -> (Db, PathBuf) {
    let db = Db::memory().await.unwrap();
    let base = scratch(name);
    db.locals().add_dir(&base).await.unwrap();
    db.locals().scan(base.to_str().unwrap(), &covers_of(&base)).await.unwrap();
    (db, base)
  }

  #[tokio::test]
  async fn a_scan_turns_audio_files_into_tracks() {
    let (db, base) = scanned("scan").await;
    let locals = db.locals();

    let titles: Vec<String> = locals.tracks(10, 0).await.unwrap().into_iter().map(|t| t.title).collect();
    assert_eq!(titles.len(), 2);
    assert!(titles.contains(&"one".to_string()) && titles.contains(&"two".to_string()));
    assert_eq!(locals.count().await.unwrap(), 2);
    let id = track_id_for(&base.join("one.mp3"));
    assert!(is_local_id(&id));
    assert_eq!(locals.path(&id).await.unwrap(), Some(base.join("one.mp3")));
    let _ = std::fs::remove_dir_all(base);
  }

  #[tokio::test]
  async fn a_second_scan_reads_only_what_changed() {
    let (db, base) = scanned("rescan").await;
    let dir = base.to_str().unwrap();
    let again = db.locals().scan(dir, &covers_of(&base)).await.unwrap();
    assert_eq!((again.added, again.removed), (0, 0));

    std::fs::remove_file(base.join("one.mp3")).unwrap();
    std::fs::write(base.join("three.ogg"), b"x").unwrap();
    let report = db.locals().scan(dir, &covers_of(&base)).await.unwrap();
    assert_eq!((report.added, report.removed), (1, 1));
    assert_eq!(db.locals().count().await.unwrap(), 2);
    assert_eq!(db.tracks().count().await.unwrap(), 2);
    let _ = std::fs::remove_dir_all(base);
  }

  #[tokio::test]
  async fn local_tracks_are_not_noted_for_sync() {
    let (db, base) = scanned("sync").await;
    db.tracks().save(&track("streamed", "S")).await.unwrap();

    let noted: Vec<String> = sqlx::query_scalar("SELECT key FROM link_dirty WHERE kind = 'track'")
      .fetch_all(db.pool())
      .await
      .unwrap();
    assert_eq!(noted, ["streamed"]);

    db.locals().remove_dir(base.to_str().unwrap()).await.unwrap();
    let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM link_dirty WHERE kind = 'track'")
      .fetch_one(db.pool())
      .await
      .unwrap();
    assert_eq!(after, 1);
    let _ = std::fs::remove_dir_all(base);
  }

  #[tokio::test]
  async fn removing_a_folder_drops_its_tracks() {
    let (db, base) = scanned("remove").await;
    assert!(db.locals().remove_dir(base.to_str().unwrap()).await.unwrap());
    assert!(!db.locals().remove_dir(base.to_str().unwrap()).await.unwrap());
    assert_eq!(db.locals().count().await.unwrap(), 0);
    assert_eq!(db.tracks().count().await.unwrap(), 0);
    assert!(base.join("one.mp3").exists());
    let _ = std::fs::remove_dir_all(base);
  }

  #[tokio::test]
  async fn an_unreadable_folder_keeps_its_tracks() {
    let (db, base) = scanned("gone").await;
    let dir = base.to_str().unwrap().to_owned();
    std::fs::remove_dir_all(&base).unwrap();

    let report = db.locals().scan(&dir, &covers_of(Path::new(&dir))).await.unwrap();
    assert!(report.unreachable);
    assert_eq!(db.locals().count().await.unwrap(), 2);
  }

  #[tokio::test]
  async fn overlapping_folders_are_refused() {
    let db = Db::memory().await.unwrap();
    let locals = db.locals();
    assert!(locals.add_dir(Path::new("/music")).await.unwrap());
    assert!(!locals.add_dir(Path::new("/music")).await.unwrap());
    for other in ["/music/rock", "/"] {
      assert!(matches!(locals.add_dir(Path::new(other)).await, Err(Error::Invalid(_))), "{other}");
    }
    assert!(locals.add_dir(Path::new("/podcasts")).await.unwrap());
    assert_eq!(locals.dirs().await.unwrap(), ["/music", "/podcasts"]);
  }

  #[tokio::test]
  async fn downloads_and_folders_make_one_list_with_separate_totals() {
    let (db, base) = scanned("usage").await;
    db.downloads().record(&track("dl", "Downloaded"), "m4a", 100).await.unwrap();
    let locals = db.locals();

    let ids: Vec<String> = locals.tracks(10, 0).await.unwrap().into_iter().map(|t| t.id).collect();
    assert_eq!(ids.len(), 3);
    assert!(ids.contains(&"dl".to_string()));
    assert_eq!(locals.count().await.unwrap(), 3);
    let usage = locals.usage().await.unwrap();
    assert_eq!(usage.downloaded, Totals { songs: 1, bytes: 100 });
    assert_eq!(usage.folders, Totals { songs: 2, bytes: 3 });
    let _ = std::fs::remove_dir_all(base);
  }

  /// A blank PNG, which is what an embedded picture is for these tests.
  fn picture(width: u32, height: u32) -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(image::RgbImage::new(width, height))
      .write_to(&mut out, image::ImageFormat::Png)
      .unwrap();
    out.into_inner()
  }

  #[test]
  fn a_picture_is_cached_once_and_shrunk() {
    let covers = std::env::temp_dir().join(format!("yamp-db-covers-{}-cache", std::process::id()));
    let _ = std::fs::remove_dir_all(&covers);
    let png = picture(1000, 1000);

    let name = cache_cover(&png, &covers).unwrap();
    assert_eq!(cache_cover(&png, &covers).as_deref(), Some(name.as_str()));
    assert_ne!(cache_cover(&picture(10, 10), &covers).unwrap(), name);

    let cached = image::open(covers.join(format!("{name}.jpg"))).unwrap();
    assert_eq!((cached.width(), cached.height()), (LOCAL_COVER_SIZE, LOCAL_COVER_SIZE));
    assert_eq!(std::fs::read_dir(&covers).unwrap().count(), 2);
    assert_eq!(cache_cover(b"not an image", &covers), None);
    let _ = std::fs::remove_dir_all(covers);
  }

  #[tokio::test]
  async fn covers_that_no_track_names_are_pruned() {
    let db = Db::memory().await.unwrap();
    let covers = std::env::temp_dir().join(format!("yamp-db-covers-{}-prune", std::process::id()));
    let _ = std::fs::remove_dir_all(&covers);
    let (kept, dropped) = (cache_cover(&picture(8, 8), &covers).unwrap(), cache_cover(&picture(9, 9), &covers).unwrap());
    let mut t = track("local-1", "T");
    t.thumbnail = Some(format!("{COVER_PREFIX}{kept}"));
    db.tracks().save(&t).await.unwrap();

    assert_eq!(db.locals().prune_covers(&covers).await.unwrap(), 1);
    assert!(covers.join(format!("{kept}.jpg")).exists());
    assert!(!covers.join(format!("{dropped}.jpg")).exists());
    let _ = std::fs::remove_dir_all(covers);
  }

  #[test]
  fn folders_are_listed_by_name_without_hidden_ones() {
    let base = scratch("browse");
    std::fs::create_dir_all(base.join("Zebra")).unwrap();
    std::fs::create_dir_all(base.join("apple")).unwrap();
    std::fs::create_dir_all(base.join(".cache")).unwrap();

    let names: Vec<String> = list_subdirs(&base).unwrap().into_iter().map(|d| d.name).collect();
    assert_eq!(names, ["apple", "sub", "Zebra"]);
    assert!(list_subdirs(&base.join("missing")).is_err());
    let _ = std::fs::remove_dir_all(base);
  }
}
