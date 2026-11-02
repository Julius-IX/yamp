use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use super::track::Track;
use crate::{clock::now_ms, Db, Error, Result};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
  pub id: String,
  pub name: String,
  pub description: Option<String>,
  pub track_count: u32,
  /// Thumbnail of the first track that has one.
  pub cover: Option<String>,
  pub created_ms: i64,
  pub updated_ms: i64,
}

impl Db {
  pub fn playlists(&self) -> Playlists {
    Playlists::new(self.pool().clone())
  }
}

/// Playlists made by the user.
#[derive(Clone)]
pub struct Playlists {
  pool: SqlitePool,
}

impl Playlists {
  pub(crate) fn new(pool: SqlitePool) -> Self {
    Self { pool }
  }

  /// Creates an empty playlist. The id is random, so playlists made on different devices
  /// never collide when they are linked.
  pub async fn create(&self, name: &str, description: Option<&str>) -> Result<Playlist> {
    let id = Uuid::new_v4().to_string();
    let now = now_ms();
    sqlx::query!(
      "INSERT INTO playlists (id, name, description, created_ms, updated_ms) VALUES (?, ?, ?, ?, ?)",
      id,
      name,
      description,
      now,
      now
    )
    .execute(&self.pool)
    .await?;
    Ok(Playlist {
      id,
      name: name.into(),
      description: description.map(Into::into),
      track_count: 0,
      cover: None,
      created_ms: now,
      updated_ms: now,
    })
  }

  /// Replaces the name and description.
  pub async fn update(&self, id: &str, name: &str, description: Option<&str>) -> Result<()> {
    let now = now_ms();
    let done = sqlx::query!(
      "UPDATE playlists SET name = ?, description = ?, updated_ms = ? WHERE id = ?",
      name,
      description,
      now,
      id
    )
    .execute(&self.pool)
    .await?;
    missing_if(done.rows_affected() == 0, id)
  }

  /// Returns whether the playlist existed. Its tracks stay in the library.
  pub async fn delete(&self, id: &str) -> Result<bool> {
    let done = sqlx::query!("DELETE FROM playlists WHERE id = ?", id)
      .execute(&self.pool)
      .await?;
    Ok(done.rows_affected() > 0)
  }

  pub async fn get(&self, id: &str) -> Result<Option<Playlist>> {
    Ok(
      sqlx::query_as!(
        Playlist,
        r#"SELECT p.id AS "id!", p.name AS "name!", p.description, p.created_ms, p.updated_ms,
             (SELECT COUNT(*) FROM playlist_tracks WHERE playlist_id = p.id) AS "track_count!: u32",
             (SELECT t.thumbnail FROM playlist_tracks x JOIN tracks t ON t.id = x.track_id
               WHERE x.playlist_id = p.id AND t.thumbnail IS NOT NULL
               ORDER BY x.position LIMIT 1) AS cover
           FROM playlists p WHERE p.id = ?"#,
        id
      )
      .fetch_optional(&self.pool)
      .await?,
    )
  }

  /// Playlists, most recently changed first.
  pub async fn list(&self) -> Result<Vec<Playlist>> {
    Ok(
      sqlx::query_as!(
        Playlist,
        r#"SELECT p.id AS "id!", p.name AS "name!", p.description, p.created_ms, p.updated_ms,
             (SELECT COUNT(*) FROM playlist_tracks WHERE playlist_id = p.id) AS "track_count!: u32",
             (SELECT t.thumbnail FROM playlist_tracks x JOIN tracks t ON t.id = x.track_id
               WHERE x.playlist_id = p.id AND t.thumbnail IS NOT NULL
               ORDER BY x.position LIMIT 1) AS cover
           FROM playlists p ORDER BY p.updated_ms DESC, p.id"#
      )
      .fetch_all(&self.pool)
      .await?,
    )
  }

  /// The tracks in playlist order. A track whose metadata has not arrived from another
  /// device yet is skipped until it does.
  pub async fn tracks(&self, id: &str) -> Result<Vec<Track>> {
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
           FROM playlist_tracks x JOIN tracks t ON t.id = x.track_id
           WHERE x.playlist_id = ? ORDER BY x.position"#,
        id
      )
      .fetch_all(&self.pool)
      .await?,
    )
  }
}

fn missing_if(missing: bool, id: &str) -> Result<()> {
  if missing {
    Err(Error::NotFound(format!("playlist {id}")))
  } else {
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use crate::{Db, Error};

  #[tokio::test]
  async fn create_list_and_update() {
    let db = Db::memory().await.unwrap();
    let playlists = db.playlists();
    let p = playlists.create("Road trip", Some("loud")).await.unwrap();

    playlists.update(&p.id, "Road trip 2", None).await.unwrap();
    let got = playlists.get(&p.id).await.unwrap().unwrap();
    assert_eq!((got.name.as_str(), got.description), ("Road trip 2", None));
    assert_eq!(playlists.list().await.unwrap().len(), 1);
    assert!(matches!(
      playlists.update("nope", "x", None).await,
      Err(Error::NotFound(_))
    ));
  }
}
