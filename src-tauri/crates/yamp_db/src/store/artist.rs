use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::{clock::now_ms, Db, Result};

/// An artist as a provider describes it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artist {
  pub id: String,
  pub name: String,
  pub thumbnail: Option<String>,
}

/// An artist the user follows and when that started.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowedArtist {
  pub id: String,
  pub name: String,
  pub thumbnail: Option<String>,
  pub followed_ms: i64,
}

impl Db {
  pub fn artists(&self) -> Artists {
    Artists::new(self.pool().clone())
  }
}

/// Followed artists.
#[derive(Clone)]
pub struct Artists {
  pool: SqlitePool,
}

impl Artists {
  pub(crate) fn new(pool: SqlitePool) -> Self {
    Self { pool }
  }

  /// Follows the artist. Following again refreshes the name and picture and keeps the
  /// original follow time. The update only runs on a change, so it leaves no trace for the
  /// sync triggers otherwise.
  pub async fn follow(&self, artist: &Artist) -> Result<()> {
    let now = now_ms();
    sqlx::query!(
      "INSERT INTO followed_artists (id, name, thumbnail, followed_ms) VALUES (?1, ?2, ?3, ?4)
       ON CONFLICT(id) DO UPDATE SET
         name = excluded.name,
         thumbnail = COALESCE(excluded.thumbnail, thumbnail)
       WHERE (name, thumbnail) IS NOT (excluded.name, COALESCE(excluded.thumbnail, thumbnail))",
      artist.id,
      artist.name,
      artist.thumbnail,
      now
    )
    .execute(&self.pool)
    .await?;
    Ok(())
  }

  /// Returns whether the artist was followed before.
  pub async fn unfollow(&self, id: &str) -> Result<bool> {
    let done = sqlx::query!("DELETE FROM followed_artists WHERE id = ?", id)
      .execute(&self.pool)
      .await?;
    Ok(done.rows_affected() > 0)
  }

  pub async fn is_following(&self, id: &str) -> Result<bool> {
    let found = sqlx::query_scalar!(
      r#"SELECT 1 AS "found!: i64" FROM followed_artists WHERE id = ?"#,
      id
    )
    .fetch_optional(&self.pool)
    .await?;
    Ok(found.is_some())
  }

  /// Followed artists by name, ignoring case.
  pub async fn list(&self) -> Result<Vec<FollowedArtist>> {
    Ok(
      sqlx::query_as!(
        FollowedArtist,
        r#"SELECT id AS "id!", name AS "name!", thumbnail, followed_ms
           FROM followed_artists ORDER BY name COLLATE NOCASE, id"#
      )
      .fetch_all(&self.pool)
      .await?,
    )
  }

  pub async fn count(&self) -> Result<u64> {
    let n = sqlx::query_scalar!(r#"SELECT COUNT(*) AS "n!: i64" FROM followed_artists"#)
      .fetch_one(&self.pool)
      .await?;
    Ok(n as u64)
  }
}

#[cfg(test)]
mod tests {
  use crate::{Artist, Db};
  use std::{thread::sleep, time::Duration};

  fn artist(id: &str, name: &str) -> Artist {
    Artist {
      id: id.into(),
      name: name.into(),
      thumbnail: None,
    }
  }

  #[tokio::test]
  async fn follow_and_unfollow() {
    let db = Db::memory().await.unwrap();
    let artists = db.artists();
    artists.follow(&artist("a", "Beta")).await.unwrap();
    artists.follow(&artist("b", "alpha")).await.unwrap();

    assert!(artists.is_following("a").await.unwrap());
    let names: Vec<String> = artists
      .list()
      .await
      .unwrap()
      .into_iter()
      .map(|a| a.name)
      .collect();
    assert_eq!(names, ["alpha", "Beta"]);

    assert!(artists.unfollow("a").await.unwrap());
    assert!(!artists.unfollow("a").await.unwrap());
    assert_eq!(artists.count().await.unwrap(), 1);
  }

  #[tokio::test]
  async fn following_again_refreshes_but_keeps_the_time() {
    let db = Db::memory().await.unwrap();
    let artists = db.artists();
    artists.follow(&artist("a", "Old Name")).await.unwrap();
    let before = artists.list().await.unwrap().remove(0);
    sleep(Duration::from_millis(3));

    let mut fresh = artist("a", "New Name");
    fresh.thumbnail = Some("pic.jpg".into());
    artists.follow(&fresh).await.unwrap();
    // a later sparse refresh keeps the picture
    artists.follow(&artist("a", "New Name")).await.unwrap();

    let after = artists.list().await.unwrap().remove(0);
    assert_eq!(after.name, "New Name");
    assert_eq!(after.thumbnail.as_deref(), Some("pic.jpg"));
    assert_eq!(after.followed_ms, before.followed_ms);
  }
}
