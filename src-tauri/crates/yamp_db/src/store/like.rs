use sqlx::SqlitePool;

use super::track::{self, Track};
use crate::{Db, Result, clock::now_ms};

impl Db {
  pub fn likes(&self) -> Likes {
    Likes::new(self.pool().clone())
  }
}

/// Liked tracks.
#[derive(Clone)]
pub struct Likes {
  pool: SqlitePool,
}

impl Likes {
  pub(crate) fn new(pool: SqlitePool) -> Self {
    Self { pool }
  }

  /// Likes the track and saves its metadata in the same transaction, so a like never
  /// points at a track the library cannot show. Liking twice keeps the first time.
  pub async fn like(&self, track: &Track) -> Result<()> {
    let mut tx = self.pool.begin().await?;
    track::upsert(&mut tx, track).await?;
    let now = now_ms();
    sqlx::query!(
      "INSERT OR IGNORE INTO likes (track_id, liked_ms) VALUES (?, ?)",
      track.id,
      now
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
  }

  /// Returns whether the track was liked before.
  pub async fn unlike(&self, track_id: &str) -> Result<bool> {
    let done = sqlx::query!("DELETE FROM likes WHERE track_id = ?", track_id)
      .execute(&self.pool)
      .await?;
    Ok(done.rows_affected() > 0)
  }

  pub async fn is_liked(&self, track_id: &str) -> Result<bool> {
    let found = sqlx::query_scalar!(
      r#"SELECT 1 AS "found!: i64" FROM likes WHERE track_id = ?"#,
      track_id
    )
    .fetch_optional(&self.pool)
    .await?;
    Ok(found.is_some())
  }

  /// Every liked id, newest first. Meant for marking hearts in lists without a query per row.
  pub async fn ids(&self) -> Result<Vec<String>> {
    Ok(
      sqlx::query_scalar!(
        r#"SELECT track_id AS "track_id!" FROM likes ORDER BY liked_ms DESC, track_id"#
      )
      .fetch_all(&self.pool)
      .await?,
    )
  }

  /// Liked tracks, newest first. A like whose metadata has not arrived from another device
  /// yet is skipped until it does.
  pub async fn list(&self, limit: u32, offset: u32) -> Result<Vec<Track>> {
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
          FROM likes l JOIN tracks t ON t.id = l.track_id
          ORDER BY l.liked_ms DESC, l.track_id LIMIT ? OFFSET ?"#,
        limit,
        offset
      )
      .fetch_all(&self.pool)
      .await?,
    )
  }

  /// Counts what [`list`](Self::list) can return.
  pub async fn count(&self) -> Result<u64> {
    let n = sqlx::query_scalar!(
      r#"SELECT COUNT(*) AS "n!: i64" FROM likes l JOIN tracks t ON t.id = l.track_id"#
    )
    .fetch_one(&self.pool)
    .await?;
    Ok(n as u64)
  }
}

#[cfg(test)]
mod tests {
  use crate::{Db, store::track::tests::track};
  use std::{thread::sleep, time::Duration};

  #[tokio::test]
  async fn like_saves_the_track_and_is_idempotent() {
    let db = Db::memory().await.unwrap();
    let likes = db.likes();
    likes.like(&track("a", "Alpha")).await.unwrap();
    let first: i64 = sqlx::query_scalar("SELECT liked_ms FROM likes")
      .fetch_one(db.pool())
      .await
      .unwrap();
    sleep(Duration::from_millis(3));
    likes.like(&track("a", "Alpha")).await.unwrap();

    let again: i64 = sqlx::query_scalar("SELECT liked_ms FROM likes")
      .fetch_one(db.pool())
      .await
      .unwrap();
    assert_eq!(first, again);
    assert!(likes.is_liked("a").await.unwrap());
    assert_eq!(db.tracks().count().await.unwrap(), 1);
    assert_eq!(likes.count().await.unwrap(), 1);
  }

  #[tokio::test]
  async fn lists_newest_first_and_pages() {
    let db = Db::memory().await.unwrap();
    let likes = db.likes();
    for id in ["a", "b", "c"] {
      likes.like(&track(id, id)).await.unwrap();
      sleep(Duration::from_millis(3));
    }

    let ids = |v: Vec<crate::Track>| v.into_iter().map(|t| t.id).collect::<Vec<_>>();
    assert_eq!(ids(likes.list(10, 0).await.unwrap()), ["c", "b", "a"]);
    assert_eq!(ids(likes.list(1, 1).await.unwrap()), ["b"]);
    assert_eq!(likes.ids().await.unwrap(), ["c", "b", "a"]);
  }

  #[tokio::test]
  async fn unlike_reports_whether_there_was_a_like() {
    let db = Db::memory().await.unwrap();
    let likes = db.likes();
    likes.like(&track("a", "Alpha")).await.unwrap();

    assert!(likes.unlike("a").await.unwrap());
    assert!(!likes.unlike("a").await.unwrap());
    assert!(!likes.is_liked("a").await.unwrap());
    // the metadata stays, other lists may still use it
    assert!(db.tracks().get("a").await.unwrap().is_some());
  }

  #[tokio::test]
  async fn a_like_without_metadata_is_hidden() {
    let db = Db::memory().await.unwrap();
    sqlx::query("INSERT INTO likes (track_id, liked_ms) VALUES ('ghost', 1)")
      .execute(db.pool())
      .await
      .unwrap();
    assert!(db.likes().list(10, 0).await.unwrap().is_empty());
    assert_eq!(db.likes().count().await.unwrap(), 0);
  }
}
