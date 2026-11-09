//! Custom covers of playlists.
//!
//! A cover is stored as a small square JPEG in the row of its playlist id, which keeps it in the
//! one place that is backed up and linked between devices.
//!
//! Only a cover that was set on purpose is stored. A playlist without one shows the thumbnail of
//! its first track, which is read from the tracks and never copied into this table.

use std::{collections::HashMap, io::Cursor};

use base64::{Engine, engine::general_purpose::STANDARD};
use image::{ExtendedColorType, ImageEncoder, ImageReader, codecs::jpeg::JpegEncoder, imageops::FilterType};
use sqlx::SqlitePool;

use crate::{Db, Error, Result, clock::now_ms};

/// Edge of a stored cover. The largest place a cover is drawn is the header of a playlist at 200
/// points, so this still looks sharp at twice the density without storing a photo.
pub const COVER_SIZE: u32 = 512;
const JPEG_QUALITY: u8 = 85;
/// An upload larger than this is refused before it is decoded.
const MAX_INPUT_BYTES: usize = 25 * 1024 * 1024;

impl Db {
  pub fn playlist_covers(&self) -> PlaylistCovers {
    PlaylistCovers { pool: self.pool().clone() }
  }
}

/// The covers that the user set for playlists. A playlist without one shows the thumbnail of its
/// first track.
#[derive(Clone)]
pub struct PlaylistCovers {
  pool: SqlitePool,
}

impl PlaylistCovers {
  /// Makes the image the cover of the playlist. Any common format is taken. The middle square is
  /// cut out and shrunk to [`COVER_SIZE`], a smaller image is kept at its own size instead of
  /// being blown up. Fails with `NotFound` when the playlist does not exist.
  pub async fn set(&self, playlist_id: &str, image: Vec<u8>) -> Result<()> {
    if image.len() > MAX_INPUT_BYTES {
      return Err(Error::Invalid(format!("the image is larger than {} MB", MAX_INPUT_BYTES / 1024 / 1024)));
    }
    let exists = sqlx::query_scalar!(r#"SELECT id AS "id!" FROM playlists WHERE id = ?"#, playlist_id)
      .fetch_optional(&self.pool)
      .await?;
    if exists.is_none() {
      return Err(Error::NotFound(format!("playlist {playlist_id}")));
    }
    // decoding and scaling a photo takes long enough to hold up the async threads
    let jpeg = tokio::task::spawn_blocking(move || scale(&image))
      .await
      .map_err(|e| Error::Invalid(format!("the image could not be processed: {e}")))??;

    let now = now_ms();
    sqlx::query!(
      "INSERT INTO playlist_covers (playlist_id, image, updated_ms) VALUES (?1, ?2, ?3)
       ON CONFLICT (playlist_id) DO UPDATE SET image = excluded.image, updated_ms = excluded.updated_ms",
      playlist_id,
      jpeg,
      now
    )
    .execute(&self.pool)
    .await?;
    Ok(())
  }

  /// The JPEG of the cover.
  pub async fn get(&self, playlist_id: &str) -> Result<Option<Vec<u8>>> {
    Ok(
      sqlx::query_scalar!("SELECT image FROM playlist_covers WHERE playlist_id = ?", playlist_id)
        .fetch_optional(&self.pool)
        .await?,
    )
  }

  /// The cover as a `data:` URL, which an `<img>` shows without a request of its own.
  pub async fn data_url(&self, playlist_id: &str) -> Result<Option<String>> {
    Ok(self.get(playlist_id).await?.map(|jpeg| format!("data:image/jpeg;base64,{}", STANDARD.encode(jpeg))))
  }

  /// Per playlist that has a cover, when the cover was set. The time changes with every new
  /// image, so it tells a reader whether the one it holds is still current. A cover whose
  /// playlist is missing, which can happen for a while during a sync, is left out.
  pub async fn versions(&self) -> Result<HashMap<String, i64>> {
    let rows = sqlx::query!(
      r#"SELECT c.playlist_id AS "playlist_id!", c.updated_ms AS "updated_ms!"
         FROM playlist_covers c JOIN playlists p ON p.id = c.playlist_id"#
    )
    .fetch_all(&self.pool)
    .await?;
    Ok(rows.into_iter().map(|r| (r.playlist_id, r.updated_ms)).collect())
  }

  /// Goes back to the thumbnail of the first track. Returns whether there was a cover.
  pub async fn remove(&self, playlist_id: &str) -> Result<bool> {
    let done = sqlx::query!("DELETE FROM playlist_covers WHERE playlist_id = ?", playlist_id)
      .execute(&self.pool)
      .await?;
    Ok(done.rows_affected() > 0)
  }
}

fn scale(bytes: &[u8]) -> Result<Vec<u8>> {
  scale_to(bytes, COVER_SIZE)
}

/// The middle square of the image as a JPEG, at most `edge` pixels wide.
pub(crate) fn scale_to(bytes: &[u8], edge: u32) -> Result<Vec<u8>> {
  let image = ImageReader::new(Cursor::new(bytes))
    .with_guessed_format()?
    .decode()
    .map_err(|e| Error::Invalid(format!("the image could not be read: {e}")))?;

  let side = image.width().min(image.height()).min(edge);
  // JPEG has no alpha, so a transparent picture is flattened here
  let square = image.resize_to_fill(side, side, FilterType::Lanczos3).to_rgb8();

  let mut out = Vec::new();
  JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY)
    .write_image(square.as_raw(), square.width(), square.height(), ExtendedColorType::Rgb8)
    .map_err(|e| Error::Invalid(format!("the cover could not be encoded: {e}")))?;
  Ok(out)
}

#[cfg(test)]
pub(crate) mod tests {
  use super::*;
  use image::{DynamicImage, ImageFormat, RgbImage};

  /// A blank PNG of the given size.
  pub(crate) fn png(width: u32, height: u32) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(RgbImage::new(width, height))
      .write_to(&mut out, ImageFormat::Png)
      .unwrap();
    out.into_inner()
  }

  #[tokio::test]
  async fn a_cover_is_a_square_of_at_most_the_cover_size() {
    let db = Db::memory().await.unwrap();
    let big = db.playlists().create("Big", None).await.unwrap();
    let small = db.playlists().create("Small", None).await.unwrap();
    db.playlist_covers().set(&big.id, png(2000, 1000)).await.unwrap();
    db.playlist_covers().set(&small.id, png(100, 60)).await.unwrap();

    let stored = image::load_from_memory(&db.playlist_covers().get(&big.id).await.unwrap().unwrap()).unwrap();
    assert_eq!((stored.width(), stored.height()), (COVER_SIZE, COVER_SIZE));
    // a small image is cropped but not enlarged
    let stored = image::load_from_memory(&db.playlist_covers().get(&small.id).await.unwrap().unwrap()).unwrap();
    assert_eq!((stored.width(), stored.height()), (60, 60));
  }

  #[tokio::test]
  async fn a_cover_can_be_replaced_removed_and_goes_with_its_playlist() {
    let db = Db::memory().await.unwrap();
    let covers = db.playlist_covers();
    let p = db.playlists().create("Mix", None).await.unwrap();
    covers.set(&p.id, png(32, 32)).await.unwrap();
    assert_eq!(covers.versions().await.unwrap().len(), 1);
    assert!(covers.data_url(&p.id).await.unwrap().unwrap().starts_with("data:image/jpeg;base64,"));

    assert!(covers.remove(&p.id).await.unwrap());
    assert!(!covers.remove(&p.id).await.unwrap());

    covers.set(&p.id, png(32, 32)).await.unwrap();
    db.playlists().delete(&p.id).await.unwrap();
    assert_eq!(covers.get(&p.id).await.unwrap(), None);
  }

  #[tokio::test]
  async fn only_a_cover_that_was_set_is_stored() {
    let db = Db::memory().await.unwrap();
    let made = db.playlists().create("Made", None).await.unwrap();
    db.playlists().save_as("album", "Album", None, &[crate::store::track::tests::track("t1", "One")]).await.unwrap();
    db.playlists().add_tracks(&made.id, &[crate::store::track::tests::track("t2", "Two")]).await.unwrap();
    assert!(db.playlist_covers().versions().await.unwrap().is_empty());
  }

  #[tokio::test]
  async fn bad_input_is_refused() {
    let db = Db::memory().await.unwrap();
    let p = db.playlists().create("Mix", None).await.unwrap();
    let covers = db.playlist_covers();
    assert!(matches!(covers.set(&p.id, b"not an image".to_vec()).await, Err(Error::Invalid(_))));
    assert!(matches!(covers.set("missing", png(8, 8)).await, Err(Error::NotFound(_))));
  }
}
