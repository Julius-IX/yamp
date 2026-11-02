use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::{
  ordered,
  track::{self, Track},
};
use crate::{Db, Result, clock::now_ms};

/// An album as a provider describes it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Album {
  pub id: String,
  pub title: String,
  pub artist: String,
  pub cover: Option<String>,
  pub release_year: Option<u32>,
}

/// An album in the library, as a list shows it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedAlbum {
  pub id: String,
  pub title: String,
  pub artist: String,
  pub cover: Option<String>,
  pub release_year: Option<u32>,
  pub track_count: u32,
  pub added_ms: i64,
}

impl Db {
  pub fn albums(&self) -> Albums {
    Albums::new(self.pool().clone())
  }
}

/// Albums saved to the library.
#[derive(Clone)]
pub struct Albums {
  pool: SqlitePool,
}

impl Albums {
  pub(crate) fn new(pool: SqlitePool) -> Self {
    Self { pool }
  }

  /// Saves the album with its tracks in order, or refreshes it when it is already saved.
  /// The track list is replaced by the given one and the original save time is kept.
  pub async fn save(&self, album: &Album, tracks: &[Track]) -> Result<()> {
    let mut tx = self.pool.begin().await?;
    for t in tracks {
      track::upsert(&mut tx, t).await?;
    }
    let now = now_ms();
    sqlx::query!(
      "INSERT INTO albums (id, title, artist, cover, release_year, added_ms)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6)
       ON CONFLICT(id) DO UPDATE SET
         title = excluded.title,
         artist = excluded.artist,
         cover = COALESCE(excluded.cover, cover),
         release_year = COALESCE(excluded.release_year, release_year)
       WHERE (title, artist, cover, release_year) IS NOT (
         excluded.title,
         excluded.artist,
         COALESCE(excluded.cover, cover),
         COALESCE(excluded.release_year, release_year)
       )",
      album.id,
      album.title,
      album.artist,
      album.cover,
      album.release_year,
      now
    )
    .execute(&mut *tx)
    .await?;
    let ids: Vec<String> = tracks.iter().map(|t| t.id.clone()).collect();
    ordered::album::replace(&mut tx, &album.id, &ids).await?;
    tx.commit().await?;
    Ok(())
  }

  /// Returns whether the album was saved. Its tracks stay in the library, they may be
  /// liked or in a playlist.
  pub async fn remove(&self, id: &str) -> Result<bool> {
    let done = sqlx::query!("DELETE FROM albums WHERE id = ?", id)
      .execute(&self.pool)
      .await?;
    Ok(done.rows_affected() > 0)
  }

  pub async fn is_saved(&self, id: &str) -> Result<bool> {
    let found = sqlx::query_scalar!(r#"SELECT 1 AS "found!: i64" FROM albums WHERE id = ?"#, id)
      .fetch_optional(&self.pool)
      .await?;
    Ok(found.is_some())
  }

  pub async fn get(&self, id: &str) -> Result<Option<SavedAlbum>> {
    Ok(
      sqlx::query_as!(
        SavedAlbum,
        r#"SELECT a.id AS "id!", a.title AS "title!", a.artist AS "artist!", a.cover,
             a.release_year AS "release_year: u32", a.added_ms,
             (SELECT COUNT(*) FROM album_tracks WHERE album_id = a.id) AS "track_count!: u32"
           FROM albums a WHERE a.id = ?"#,
        id
      )
      .fetch_optional(&self.pool)
      .await?,
    )
  }

  /// Saved albums, most recently saved first.
  pub async fn list(&self) -> Result<Vec<SavedAlbum>> {
    Ok(
      sqlx::query_as!(
        SavedAlbum,
        r#"SELECT a.id AS "id!", a.title AS "title!", a.artist AS "artist!", a.cover,
             a.release_year AS "release_year: u32", a.added_ms,
             (SELECT COUNT(*) FROM album_tracks WHERE album_id = a.id) AS "track_count!: u32"
           FROM albums a ORDER BY a.added_ms DESC, a.id"#
      )
      .fetch_all(&self.pool)
      .await?,
    )
  }

  /// The tracks of an album in album order. A track whose metadata has not arrived from
  /// another device yet is skipped until it does.
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
          FROM album_tracks x JOIN tracks t ON t.id = x.track_id
           WHERE x.album_id = ? ORDER BY x.position"#,
        id
      )
      .fetch_all(&self.pool)
      .await?,
    )
  }
}

#[cfg(test)]
mod tests {
  use crate::{Album, Db, store::track::tests::track};

  fn album(id: &str) -> Album {
    Album {
      id: id.into(),
      title: format!("Album {id}"),
      artist: "Artist".into(),
      cover: None,
      release_year: Some(2001),
    }
  }

  fn ids(tracks: Vec<crate::Track>) -> Vec<String> {
    tracks.into_iter().map(|t| t.id).collect()
  }

  #[tokio::test]
  async fn saves_with_ordered_tracks() {
    let db = Db::memory().await.unwrap();
    let albums = db.albums();
    let tracks = [track("c", "C"), track("a", "A"), track("b", "B")];
    albums.save(&album("x"), &tracks).await.unwrap();

    assert_eq!(ids(albums.tracks("x").await.unwrap()), ["c", "a", "b"]);
    let saved = albums.get("x").await.unwrap().unwrap();
    assert_eq!((saved.track_count, saved.release_year), (3, Some(2001)));
    assert!(albums.is_saved("x").await.unwrap());
    assert_eq!(db.tracks().count().await.unwrap(), 3);
  }

  #[tokio::test]
  async fn saving_again_replaces_the_tracks_and_keeps_the_time() {
    let db = Db::memory().await.unwrap();
    let albums = db.albums();
    albums
      .save(&album("x"), &[track("a", "A"), track("b", "B")])
      .await
      .unwrap();
    let before = albums.get("x").await.unwrap().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(3));

    albums
      .save(&album("x"), &[track("b", "B"), track("c", "C")])
      .await
      .unwrap();
    let after = albums.get("x").await.unwrap().unwrap();
    assert_eq!(after.added_ms, before.added_ms);
    assert_eq!(ids(albums.tracks("x").await.unwrap()), ["b", "c"]);
  }

  #[tokio::test]
  async fn remove_drops_the_list_but_not_the_tracks() {
    let db = Db::memory().await.unwrap();
    let albums = db.albums();
    albums.save(&album("x"), &[track("a", "A")]).await.unwrap();

    assert!(albums.remove("x").await.unwrap());
    assert!(!albums.remove("x").await.unwrap());
    let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM album_tracks")
      .fetch_one(db.pool())
      .await
      .unwrap();
    assert_eq!(left, 0);
    assert!(db.tracks().get("a").await.unwrap().is_some());
  }

  #[tokio::test]
  async fn lists_most_recent_first() {
    let db = Db::memory().await.unwrap();
    let albums = db.albums();
    for id in ["x", "y"] {
      albums.save(&album(id), &[]).await.unwrap();
      std::thread::sleep(std::time::Duration::from_millis(3));
    }
    let order: Vec<String> = albums
      .list()
      .await
      .unwrap()
      .into_iter()
      .map(|a| a.id)
      .collect();
    assert_eq!(order, ["y", "x"]);
  }
}
