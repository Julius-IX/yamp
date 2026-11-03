use serde::{Deserialize, Serialize};
use sqlx::{SqliteConnection, SqlitePool};

use super::track::{self, Track};
use crate::{clock::now_ms, Db, Error, Result};

/// The longest id that is accepted. Leaves room for the extension under the 255 byte
/// limit that file names have on most file systems.
const MAX_ID_LEN: usize = 200;
const MAX_EXT_LEN: usize = 8;

/// A downloaded audio file. Its name on disk is `track_id.ext`, see [`Download::file_name`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Download {
  pub track_id: String,
  pub ext: String,
  pub size_bytes: u64,
  pub downloaded_ms: i64,
}

impl Download {
  pub fn file_name(&self) -> String {
    format!("{}.{}", self.track_id, self.ext)
  }
}

/// Ids become file names, so only characters that every file system accepts are allowed.
/// Ids that differ only in case would share a file on macOS and Windows. The ids that
/// providers hand out are long and random enough for that not to happen in practice.
pub(crate) fn check_name(id: &str, ext: &str) -> Result<()> {
  let id_ok = !id.is_empty()
    && id.len() <= MAX_ID_LEN
    && id
      .bytes()
      .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
  let ext_ok = !ext.is_empty()
    && ext.len() <= MAX_EXT_LEN
    && ext
      .bytes()
      .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit());
  if id_ok && ext_ok {
    Ok(())
  } else {
    Err(Error::Invalid(format!(
      "{id:?}.{ext:?} is not a usable file name"
    )))
  }
}

impl Db {
  pub fn downloads(&self) -> Downloads {
    Downloads::new(self.pool().clone())
  }
}

pub(crate) async fn find(conn: &mut SqliteConnection, track_id: &str) -> Result<Option<Download>> {
  Ok(
    sqlx::query_as!(
      Download,
      r#"SELECT track_id AS "track_id!", ext AS "ext!", size_bytes AS "size_bytes: u64", downloaded_ms
         FROM downloads WHERE track_id = ?"#,
      track_id
    )
    .fetch_optional(conn)
    .await?,
  )
}

/// Records of downloaded audio. The files themselves are handled by `MediaStore`.
#[derive(Clone)]
pub struct Downloads {
  pool: SqlitePool,
}

impl Downloads {
  pub(crate) fn new(pool: SqlitePool) -> Self {
    Self { pool }
  }

  /// Records that the track is on disk, saving its metadata in the same transaction.
  /// Returns the record it replaced, which tells the caller about an old file to remove.
  pub async fn record(
    &self,
    track: &Track,
    ext: &str,
    size_bytes: u64,
  ) -> Result<Option<Download>> {
    check_name(&track.id, ext)?;
    let mut tx = self.pool.begin().await?;
    track::upsert(&mut tx, track).await?;
    let previous = find(&mut tx, &track.id).await?;
    let (size, now) = (size_bytes as i64, now_ms());
    sqlx::query!(
      "INSERT INTO downloads (track_id, ext, size_bytes, downloaded_ms) VALUES (?1, ?2, ?3, ?4)
       ON CONFLICT(track_id) DO UPDATE SET
         ext = excluded.ext, size_bytes = excluded.size_bytes, downloaded_ms = excluded.downloaded_ms",
      track.id,
      ext,
      size,
      now
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(previous)
  }

  pub async fn get(&self, track_id: &str) -> Result<Option<Download>> {
    let mut conn = self.pool.acquire().await?;
    find(&mut conn, track_id).await
  }

  pub async fn is_downloaded(&self, track_id: &str) -> Result<bool> {
    Ok(self.get(track_id).await?.is_some())
  }

  /// Forgets the record and returns it. Removing the file is up to the caller.
  pub async fn remove(&self, track_id: &str) -> Result<Option<Download>> {
    let mut tx = self.pool.begin().await?;
    let found = find(&mut tx, track_id).await?;
    sqlx::query!("DELETE FROM downloads WHERE track_id = ?", track_id)
      .execute(&mut *tx)
      .await?;
    tx.commit().await?;
    Ok(found)
  }

  /// Every record, newest first.
  pub async fn list(&self) -> Result<Vec<Download>> {
    Ok(
      sqlx::query_as!(
        Download,
        r#"SELECT track_id AS "track_id!", ext AS "ext!", size_bytes AS "size_bytes: u64", downloaded_ms
           FROM downloads ORDER BY downloaded_ms DESC, track_id"#
      )
      .fetch_all(&self.pool)
      .await?,
    )
  }

  /// Downloaded tracks, newest first. A record whose metadata is missing is skipped.
  pub async fn tracks(&self, limit: u32, offset: u32) -> Result<Vec<Track>> {
    Ok(
      sqlx::query_as!(
        Track,
        r#"SELECT
             t.id AS "id!",
             t.title AS "title!",
             t.artist AS "artist!",
             t.album,
             t.duration_secs,
             t.thumbnail,
             t.release_year AS "release_year: u32"
           FROM downloads d JOIN tracks t ON t.id = d.track_id
           ORDER BY d.downloaded_ms DESC, d.track_id LIMIT ? OFFSET ?"#,
        limit,
        offset
      )
      .fetch_all(&self.pool)
      .await?,
    )
  }

  /// Bytes taken by all recorded files.
  pub async fn total_size(&self) -> Result<u64> {
    let n =
      sqlx::query_scalar!(r#"SELECT COALESCE(SUM(size_bytes), 0) AS "n!: i64" FROM downloads"#)
        .fetch_one(&self.pool)
        .await?;
    Ok(n as u64)
  }

  pub async fn count(&self) -> Result<u64> {
    let n = sqlx::query_scalar!(r#"SELECT COUNT(*) AS "n!: i64" FROM downloads"#)
      .fetch_one(&self.pool)
      .await?;
    Ok(n as u64)
  }
}

#[cfg(test)]
mod tests {
  use crate::{store::track::tests::track, Db, Error};

  #[tokio::test]
  async fn records_and_totals() {
    let db = Db::memory().await.unwrap();
    let downloads = db.downloads();
    assert!(downloads
      .record(&track("a", "A"), "m4a", 100)
      .await
      .unwrap()
      .is_none());
    downloads
      .record(&track("b", "B"), "opus", 50)
      .await
      .unwrap();

    assert_eq!(downloads.total_size().await.unwrap(), 150);
    assert_eq!(downloads.count().await.unwrap(), 2);
    let got = downloads.get("a").await.unwrap().unwrap();
    assert_eq!(got.file_name(), "a.m4a");
    assert!(downloads.is_downloaded("b").await.unwrap());
    assert_eq!(db.tracks().count().await.unwrap(), 2);
  }

  #[tokio::test]
  async fn recording_again_returns_the_old_record() {
    let db = Db::memory().await.unwrap();
    let downloads = db.downloads();
    downloads
      .record(&track("a", "A"), "webm", 10)
      .await
      .unwrap();

    let old = downloads
      .record(&track("a", "A"), "m4a", 20)
      .await
      .unwrap()
      .unwrap();
    assert_eq!((old.ext.as_str(), old.size_bytes), ("webm", 10));
    assert_eq!(downloads.get("a").await.unwrap().unwrap().ext, "m4a");
    assert_eq!(downloads.count().await.unwrap(), 1);
  }

  #[tokio::test]
  async fn remove_returns_the_record() {
    let db = Db::memory().await.unwrap();
    let downloads = db.downloads();
    downloads.record(&track("a", "A"), "m4a", 10).await.unwrap();

    assert_eq!(downloads.remove("a").await.unwrap().unwrap().track_id, "a");
    assert_eq!(downloads.remove("a").await.unwrap(), None);
    assert_eq!(downloads.total_size().await.unwrap(), 0);
  }

  #[tokio::test]
  async fn lists_newest_first() {
    let db = Db::memory().await.unwrap();
    let downloads = db.downloads();
    for id in ["a", "b", "c"] {
      downloads.record(&track(id, id), "m4a", 1).await.unwrap();
      std::thread::sleep(std::time::Duration::from_millis(3));
    }
    let ids: Vec<String> = downloads
      .tracks(10, 0)
      .await
      .unwrap()
      .into_iter()
      .map(|t| t.id)
      .collect();
    assert_eq!(ids, ["c", "b", "a"]);
    assert_eq!(downloads.list().await.unwrap().len(), 3);
  }

  #[tokio::test]
  async fn refuses_names_that_are_not_safe_on_disk() {
    let db = Db::memory().await.unwrap();
    let downloads = db.downloads();
    for (id, ext) in [
      ("../x", "m4a"),
      ("a/b", "m4a"),
      ("", "m4a"),
      ("a", "M4A"),
      ("a", ""),
      ("a", "m.4"),
    ] {
      assert!(
        matches!(
          downloads.record(&track(id, "t"), ext, 1).await,
          Err(Error::Invalid(_))
        ),
        "{id:?} {ext:?}"
      );
    }
    assert_eq!(downloads.count().await.unwrap(), 0);
  }
}
