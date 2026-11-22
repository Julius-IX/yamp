//! Custom covers of single tracks.
//!
//! The thumbnail that a track comes with is often a video frame with black bars, or a picture that
//! does not suit the song. A cover that was set on purpose wins over it wherever the track is drawn.
//!
//! A cover is a small square JPEG, like the ones of playlists, but it is not part of the sync. The
//! thumbnail of a track travels with its metadata, and a second place that changes what it shows
//! would need an entity of its own in `link`.

use base64::{Engine, engine::general_purpose::STANDARD};
use sqlx::{Row, SqlitePool};

use super::cover::{self, COVER_SIZE};
use crate::{Db, Error, Result, clock::now_ms};

/// An upload larger than this is refused before it is decoded.
const MAX_INPUT_BYTES: usize = 25 * 1024 * 1024;

impl Db {
  pub fn track_covers(&self) -> TrackCovers {
    TrackCovers { pool: self.pool().clone() }
  }
}

fn data_url(jpeg: &[u8]) -> String {
  format!("data:image/jpeg;base64,{}", STANDARD.encode(jpeg))
}

#[derive(Clone)]
pub struct TrackCovers {
  pool: SqlitePool,
}

impl TrackCovers {
  /// Makes the image the cover of the track and returns it as a `data:` URL. Any common format is
  /// taken, and the middle square is cut out and shrunk. The track needs no row in the library.
  pub async fn set(&self, track_id: &str, image: Vec<u8>) -> Result<String> {
    if image.len() > MAX_INPUT_BYTES {
      return Err(Error::Invalid(format!("the image is larger than {} MB", MAX_INPUT_BYTES / 1024 / 1024)));
    }
    // decoding and scaling a photo takes long enough to hold up the async threads
    let jpeg = tokio::task::spawn_blocking(move || cover::scale_to(&image, COVER_SIZE))
      .await
      .map_err(|e| Error::Invalid(format!("the image could not be processed: {e}")))??;
    sqlx::query(
      "INSERT INTO track_covers (track_id, image, updated_ms) VALUES (?1, ?2, ?3)
       ON CONFLICT (track_id) DO UPDATE SET image = excluded.image, updated_ms = excluded.updated_ms",
    )
    .bind(track_id)
    .bind(&jpeg)
    .bind(now_ms())
    .execute(&self.pool)
    .await?;
    Ok(data_url(&jpeg))
  }

  /// Goes back to the thumbnail of the track. Returns whether there was a cover.
  pub async fn remove(&self, track_id: &str) -> Result<bool> {
    let done = sqlx::query("DELETE FROM track_covers WHERE track_id = ?1")
      .bind(track_id)
      .execute(&self.pool)
      .await?;
    Ok(done.rows_affected() > 0)
  }

  /// Every cover as the id of its track and a `data:` URL.
  pub async fn all(&self) -> Result<Vec<(String, String)>> {
    let rows = sqlx::query("SELECT track_id, image FROM track_covers ORDER BY updated_ms")
      .fetch_all(&self.pool)
      .await?;
    rows
      .iter()
      .map(|r| {
        let jpeg: Vec<u8> = r.try_get("image")?;
        Ok((r.try_get("track_id")?, data_url(&jpeg)))
      })
      .collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::store::cover::tests::png;

  #[tokio::test]
  async fn a_cover_is_stored_scaled_and_replaced() {
    let db = Db::memory().await.unwrap();
    let covers = db.track_covers();

    let url = covers.set("t1", png(900, 600)).await.unwrap();
    assert!(url.starts_with("data:image/jpeg;base64,"));
    let all = covers.all().await.unwrap();
    assert_eq!(all, [("t1".to_string(), url)]);

    covers.set("t1", png(40, 40)).await.unwrap();
    assert_eq!(covers.all().await.unwrap().len(), 1);
  }

  #[tokio::test]
  async fn removing_a_cover_says_whether_there_was_one() {
    let db = Db::memory().await.unwrap();
    let covers = db.track_covers();
    covers.set("t1", png(10, 10)).await.unwrap();
    assert!(covers.remove("t1").await.unwrap());
    assert!(!covers.remove("t1").await.unwrap());
    assert!(covers.all().await.unwrap().is_empty());
  }

  #[tokio::test]
  async fn something_that_is_not_an_image_is_refused() {
    let db = Db::memory().await.unwrap();
    let covers = db.track_covers();
    assert!(matches!(covers.set("t1", b"nope".to_vec()).await, Err(Error::Invalid(_))));
    assert!(covers.all().await.unwrap().is_empty());
  }
}
