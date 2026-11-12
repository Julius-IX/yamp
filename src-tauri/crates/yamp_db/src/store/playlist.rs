use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sqlx::{SqliteConnection, SqlitePool};
use uuid::Uuid;

use super::{
  ordered,
  track::{self, Track},
};
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

  /// Creates a playlist under the given id and fills it, unless one with that id exists.
  /// An album saved to the library takes the album id, so the album can be looked up by it and
  /// saving it again, also from another device, cannot make a second copy. Returns whether a
  /// playlist was made. One that already exists is left as it is, since it may have been edited.
  pub async fn save_as(
    &self,
    id: &str,
    name: &str,
    description: Option<&str>,
    tracks: &[Track],
  ) -> Result<bool> {
    let mut tx = self.pool.begin().await?;
    let now = now_ms();
    let done = sqlx::query!(
      "INSERT INTO playlists (id, name, description, created_ms, updated_ms)
       VALUES (?, ?, ?, ?, ?) ON CONFLICT (id) DO NOTHING",
      id,
      name,
      description,
      now,
      now
    )
    .execute(&mut *tx)
    .await?;
    if done.rows_affected() == 0 {
      return Ok(false);
    }
    for t in tracks {
      track::upsert(&mut tx, t).await?;
    }
    let ids: Vec<String> = tracks.iter().map(|t| t.id.clone()).collect();
    ordered::playlist::replace(&mut tx, id, &ids).await?;
    tx.commit().await?;
    Ok(true)
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
        r#"SELECT t.id AS "id!", t.title AS "title!", t.artist AS "artist!", t.album, t.duration_secs, t.thumbnail, t.release_year AS "release_year: u32" FROM playlist_tracks x JOIN tracks t ON t.id = x.track_id
           WHERE x.playlist_id = ? ORDER BY x.position"#,
        id
      )
      .fetch_all(&self.pool)
      .await?,
    )
  }

  /// Appends tracks to the end. Adding a track that is already in the list adds it again.
  pub async fn add_tracks(&self, id: &str, tracks: &[Track]) -> Result<()> {
    let mut tx = self.pool.begin().await?;
    touch(&mut tx, id).await?;
    for t in tracks {
      track::upsert(&mut tx, t).await?;
    }
    let mut ids = ordered::playlist::ids(&mut tx, id).await?;
    ids.extend(tracks.iter().map(|t| t.id.clone()));
    ordered::playlist::replace(&mut tx, id, &ids).await?;
    tx.commit().await?;
    Ok(())
  }

  /// Removes the entry at `position`, counting from 0.
  pub async fn remove_at(&self, id: &str, position: u32) -> Result<()> {
    self
      .edit(id, |ids| {
        let at = position as usize;
        if at >= ids.len() {
          return Err(Error::Invalid(format!("no entry at position {position}")));
        }
        ids.remove(at);
        Ok(())
      })
      .await
  }

  /// Removes the entries at all of `positions` in one edit. Order and repeats in `positions`
  /// do not matter. Nothing is removed when one of them is out of range.
  pub async fn remove_positions(&self, id: &str, positions: &[u32]) -> Result<()> {
    self
      .edit(id, |ids| {
        let doomed: BTreeSet<usize> = positions.iter().map(|&p| p as usize).collect();
        if let Some(bad) = doomed.iter().find(|&&p| p >= ids.len()) {
          return Err(Error::Invalid(format!("no entry at position {bad}")));
        }
        let mut at = 0;
        ids.retain(|_| {
          at += 1;
          !doomed.contains(&(at - 1))
        });
        Ok(())
      })
      .await
  }

  /// Moves the entry at `from` so that it ends up at `to`.
  pub async fn move_track(&self, id: &str, from: u32, to: u32) -> Result<()> {
    self
      .edit(id, |ids| {
        let (from, to) = (from as usize, to as usize);
        if from >= ids.len() || to >= ids.len() {
          return Err(Error::Invalid(format!("cannot move {from} to {to}")));
        }
        let moved = ids.remove(from);
        ids.insert(to, moved);
        Ok(())
      })
      .await
  }

  async fn edit(
    &self,
    id: &str,
    change: impl FnOnce(&mut Vec<String>) -> Result<()>,
  ) -> Result<()> {
    let mut tx = self.pool.begin().await?;
    touch(&mut tx, id).await?;
    let mut ids = ordered::playlist::ids(&mut tx, id).await?;
    change(&mut ids)?;
    ordered::playlist::replace(&mut tx, id, &ids).await?;
    tx.commit().await?;
    Ok(())
  }
}

/// Bumps the change time. It runs first in every edit, so a missing playlist fails
/// before anything else is written.
async fn touch(conn: &mut SqliteConnection, id: &str) -> Result<()> {
  let now = now_ms();
  let done = sqlx::query!("UPDATE playlists SET updated_ms = ? WHERE id = ?", now, id)
    .execute(conn)
    .await?;
  missing_if(done.rows_affected() == 0, id)
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
  use crate::{store::track::tests::track, Db, Error};

  async fn ids(db: &Db, id: &str) -> Vec<String> {
    let tracks = db.playlists().tracks(id).await.unwrap();
    tracks.into_iter().map(|t| t.id).collect()
  }

  async fn filled(db: &Db) -> String {
    let p = db.playlists().create("Mix", None).await.unwrap();
    let tracks: Vec<_> = ["a", "b", "c", "d"].map(|i| track(i, i)).into();
    db.playlists().add_tracks(&p.id, &tracks).await.unwrap();
    p.id
  }

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

  #[tokio::test]
  async fn save_as_creates_once_and_keeps_later_edits() {
    let db = Db::memory().await.unwrap();
    let playlists = db.playlists();
    let tracks = [track("a", "a"), track("b", "b")];
    assert!(playlists.save_as("alb", "Album", Some("Artist"), &tracks).await.unwrap());
    assert_eq!(ids(&db, "alb").await, ["a", "b"]);

    playlists.update("alb", "Renamed", None).await.unwrap();
    playlists.remove_at("alb", 0).await.unwrap();
    assert!(!playlists.save_as("alb", "Album", Some("Artist"), &tracks).await.unwrap());

    let got = playlists.get("alb").await.unwrap().unwrap();
    assert_eq!(got.name, "Renamed");
    assert_eq!(ids(&db, "alb").await, ["b"]);
  }

  #[tokio::test]
  async fn appends_in_order_and_allows_repeats() {
    let db = Db::memory().await.unwrap();
    let id = filled(&db).await;
    db.playlists()
      .add_tracks(&id, &[track("a", "a")])
      .await
      .unwrap();

    assert_eq!(ids(&db, &id).await, ["a", "b", "c", "d", "a"]);
    assert_eq!(
      db.playlists().get(&id).await.unwrap().unwrap().track_count,
      5
    );
  }

  #[tokio::test]
  async fn moves_in_both_directions() {
    let db = Db::memory().await.unwrap();
    let id = filled(&db).await;
    let playlists = db.playlists();

    playlists.move_track(&id, 0, 2).await.unwrap();
    assert_eq!(ids(&db, &id).await, ["b", "c", "a", "d"]);
    playlists.move_track(&id, 3, 0).await.unwrap();
    assert_eq!(ids(&db, &id).await, ["d", "b", "c", "a"]);
    assert!(matches!(
      playlists.move_track(&id, 0, 9).await,
      Err(Error::Invalid(_))
    ));
  }

  #[tokio::test]
  async fn removes_by_position() {
    let db = Db::memory().await.unwrap();
    let id = filled(&db).await;
    db.playlists().remove_at(&id, 1).await.unwrap();

    assert_eq!(ids(&db, &id).await, ["a", "c", "d"]);
    assert!(db.playlists().remove_at(&id, 3).await.is_err());
  }

  #[tokio::test]
  async fn removes_several_positions_at_once() {
    let db = Db::memory().await.unwrap();
    let id = filled(&db).await;
    let playlists = db.playlists();

    // out of order and repeated on purpose
    playlists.remove_positions(&id, &[3, 0, 3]).await.unwrap();
    assert_eq!(ids(&db, &id).await, ["b", "c"]);
    assert!(playlists.remove_positions(&id, &[0, 2]).await.is_err());
    assert_eq!(ids(&db, &id).await, ["b", "c"]);
  }

  #[tokio::test]
  async fn delete_removes_the_list_and_keeps_tracks() {
    let db = Db::memory().await.unwrap();
    let id = filled(&db).await;

    assert!(db.playlists().delete(&id).await.unwrap());
    assert!(!db.playlists().delete(&id).await.unwrap());
    assert!(ids(&db, &id).await.is_empty());
    assert_eq!(db.tracks().count().await.unwrap(), 4);
  }

  #[tokio::test]
  async fn cover_is_the_first_thumbnail() {
    let db = Db::memory().await.unwrap();
    let p = db.playlists().create("Mix", None).await.unwrap();
    let mut b = track("b", "b");
    b.thumbnail = Some("b.jpg".into());
    db.playlists()
      .add_tracks(&p.id, &[track("a", "a"), b])
      .await
      .unwrap();

    let got = db.playlists().get(&p.id).await.unwrap().unwrap();
    assert_eq!(got.cover.as_deref(), Some("b.jpg"));
  }
}
