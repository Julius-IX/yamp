use serde::{Deserialize, Serialize};
use sqlx::{SqliteConnection, SqlitePool};

use crate::{Db, Result};

/// A playable item, shaped like `yamp_media::Track` so the two convert field by field.
/// Only `id` and `title` are guaranteed by providers, the rest is best effort.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
  pub id: String,
  pub title: String,
  pub artist: String,
  pub album: Option<String>,
  pub duration_secs: Option<f64>,
  pub thumbnail: Option<String>,
  pub release_year: Option<u32>,
}

impl Db {
  pub fn tracks(&self) -> Tracks {
    Tracks::new(self.pool().clone())
  }
}

/// Saves the track, or refreshes it when the id is known. A field that is missing on the
/// new value keeps what is stored, because providers differ in how much a listing exposes
/// and a sparse search result must not wipe the details of a full one.
///
/// The update only runs when something differs. A save that changes nothing then leaves
/// no trace for the sync triggers.
pub(crate) async fn upsert(conn: &mut SqliteConnection, track: &Track) -> Result<()> {
  sqlx::query!(
    "INSERT INTO tracks (id, title, artist, album, duration_secs, thumbnail, release_year)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
     ON CONFLICT(id) DO UPDATE SET
       title = excluded.title,
       artist = excluded.artist,
       album = COALESCE(excluded.album, album),
       duration_secs = COALESCE(excluded.duration_secs, duration_secs),
       thumbnail = COALESCE(excluded.thumbnail, thumbnail),
       release_year = COALESCE(excluded.release_year, release_year)
     WHERE (title, artist, album, duration_secs, thumbnail, release_year) IS NOT (
       excluded.title,
       excluded.artist,
       COALESCE(excluded.album, album),
       COALESCE(excluded.duration_secs, duration_secs),
       COALESCE(excluded.thumbnail, thumbnail),
       COALESCE(excluded.release_year, release_year)
     )",
    track.id,
    track.title,
    track.artist,
    track.album,
    track.duration_secs,
    track.thumbnail,
    track.release_year,
  )
  .execute(conn)
  .await?;
  Ok(())
}

/// Track metadata. Other stores save the tracks they are given on their own, so this is
/// mostly for reading and searching what the library already knows.
#[derive(Clone)]
pub struct Tracks {
  pool: SqlitePool,
}

impl Tracks {
  pub(crate) fn new(pool: SqlitePool) -> Self {
    Self { pool }
  }

  pub async fn save(&self, track: &Track) -> Result<()> {
    let mut conn = self.pool.acquire().await?;
    upsert(&mut conn, track).await
  }

  /// Saves all tracks in one transaction, so a failure keeps none of them.
  pub async fn save_many(&self, tracks: &[Track]) -> Result<()> {
    let mut tx = self.pool.begin().await?;
    for track in tracks {
      upsert(&mut tx, track).await?;
    }
    tx.commit().await?;
    Ok(())
  }

  pub async fn get(&self, id: &str) -> Result<Option<Track>> {
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
        FROM tracks t WHERE t.id = ?"#,
        id
      )
      .fetch_optional(&self.pool)
      .await?,
    )
  }

  /// Tracks in the order of `ids`. Ids that are not stored are left out.
  pub async fn get_many(&self, ids: &[String]) -> Result<Vec<Track>> {
    let mut out = Vec::with_capacity(ids.len());
    let mut conn = self.pool.acquire().await?;
    for id in ids {
      let found = sqlx::query_as!(
        Track,
        r#"SELECT t.id AS "id!",
            t.title AS "title!",
            t.artist AS "artist!",
            t.album,
            t.duration_secs,
            t.thumbnail,
            t.release_year AS "release_year: u32"
        FROM tracks t WHERE t.id = ?"#,
        id
      )
      .fetch_optional(&mut *conn)
      .await?;
      out.extend(found);
    }
    Ok(out)
  }

  /// Tracks whose title, artist or album contains `text`, best matches by title first.
  pub async fn search(&self, text: &str, limit: u32) -> Result<Vec<Track>> {
    // `%`, `_` and the escape itself are literal in what the user typed
    let like = format!(
      "%{}%",
      text
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
    );
    Ok(
      sqlx::query_as!(
        Track,
        r#"SELECT t.id AS "id!", t.title AS "title!", t.artist AS "artist!", t.album, t.duration_secs, t.thumbnail, t.release_year AS "release_year: u32" FROM tracks t
           WHERE t.title LIKE ?1 ESCAPE '\' OR t.artist LIKE ?1 ESCAPE '\' OR t.album LIKE ?1 ESCAPE '\'
           ORDER BY (t.title LIKE ?1 ESCAPE '\') DESC, t.title COLLATE NOCASE
           LIMIT ?2"#,
        like,
        limit
      )
      .fetch_all(&self.pool)
      .await?,
    )
  }

  pub async fn count(&self) -> Result<u64> {
    let n = sqlx::query_scalar!(r#"SELECT COUNT(*) AS "n!: i64" FROM tracks"#)
      .fetch_one(&self.pool)
      .await?;
    Ok(n as u64)
  }
}

#[cfg(test)]
pub(crate) mod tests {
  use crate::{Db, Track};

  pub(crate) fn track(id: &str, title: &str) -> Track {
    Track {
      id: id.into(),
      title: title.into(),
      artist: "Artist".into(),
      album: None,
      duration_secs: None,
      thumbnail: None,
      release_year: None,
    }
  }

  #[tokio::test]
  async fn saves_and_reads_back() {
    let db = Db::memory().await.unwrap();
    let tracks = db.tracks();
    let mut t = track("a", "Alpha");
    t.release_year = Some(1999);
    t.duration_secs = Some(12.5);
    tracks.save(&t).await.unwrap();

    assert_eq!(tracks.get("a").await.unwrap(), Some(t));
    assert_eq!(tracks.get("missing").await.unwrap(), None);
    assert_eq!(tracks.count().await.unwrap(), 1);
  }

  #[tokio::test]
  async fn a_sparse_save_keeps_known_details() {
    let db = Db::memory().await.unwrap();
    let tracks = db.tracks();
    let mut full = track("a", "Alpha");
    full.album = Some("First".into());
    full.thumbnail = Some("cover.jpg".into());
    tracks.save(&full).await.unwrap();

    let mut sparse = track("a", "Alpha (Remaster)");
    sparse.artist = "Someone Else".into();
    tracks.save(&sparse).await.unwrap();

    let got = tracks.get("a").await.unwrap().unwrap();
    assert_eq!(got.title, "Alpha (Remaster)");
    assert_eq!(got.artist, "Someone Else");
    assert_eq!(got.album.as_deref(), Some("First"));
    assert_eq!(got.thumbnail.as_deref(), Some("cover.jpg"));
  }

  #[tokio::test]
  async fn get_many_follows_the_given_order() {
    let db = Db::memory().await.unwrap();
    let tracks = db.tracks();
    tracks
      .save_many(&[track("a", "A"), track("b", "B"), track("c", "C")])
      .await
      .unwrap();

    let ids: Vec<String> = ["c", "x", "a"].map(String::from).into();
    let got: Vec<String> = tracks
      .get_many(&ids)
      .await
      .unwrap()
      .into_iter()
      .map(|t| t.id)
      .collect();
    assert_eq!(got, ["c", "a"]);
  }

  #[tokio::test]
  async fn search_matches_literally() {
    let db = Db::memory().await.unwrap();
    let tracks = db.tracks();
    tracks
      .save_many(&[
        track("a", "100% Pure"),
        track("b", "Plain"),
        track("c", "Pure Love"),
      ])
      .await
      .unwrap();

    let hits = |q: &'static str| {
      let tracks = tracks.clone();
      async move {
        tracks
          .search(q, 10)
          .await
          .unwrap()
          .into_iter()
          .map(|t| t.id)
          .collect::<Vec<_>>()
      }
    };
    assert_eq!(hits("100%").await, ["a"]);
    assert_eq!(hits("pure").await, ["a", "c"]);
    assert_eq!(hits("_").await, Vec::<String>::new());
  }
}
