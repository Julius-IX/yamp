//! Track lists that keep an order, shared by albums and playlists.
//!
//! A list is stored as rows `(owner, position, track_id)` with positions counting from 0
//! and no gaps. Edits rewrite the whole list instead of shifting positions in place.
//! SQLite checks a primary key per row while an `UPDATE` runs, so `position = position + 1`
//! can hit a row that has not moved yet. Lists are short enough that a rewrite is cheap.
//!
//! The queries are written out once per table, since the checked macros need the SQL as a
//! literal and cannot take a table name.

use sqlx::SqliteConnection;

use crate::Result;

pub(crate) mod album {
  use super::*;

  pub(crate) async fn ids(conn: &mut SqliteConnection, album_id: &str) -> Result<Vec<String>> {
    Ok(
      sqlx::query_scalar!(
        r#"SELECT track_id AS "track_id!" FROM album_tracks WHERE album_id = ? ORDER BY position"#,
        album_id
      )
      .fetch_all(conn)
      .await?,
    )
  }

  /// Makes the stored list equal to `wanted`. Nothing is written when it already is, so
  /// saving an unchanged list leaves no trace for the sync triggers. Returns whether
  /// anything changed.
  pub(crate) async fn replace(
    conn: &mut SqliteConnection,
    album_id: &str,
    wanted: &[String],
  ) -> Result<bool> {
    if ids(&mut *conn, album_id).await? == wanted {
      return Ok(false);
    }
    sqlx::query!("DELETE FROM album_tracks WHERE album_id = ?", album_id)
      .execute(&mut *conn)
      .await?;
    for (position, track_id) in wanted.iter().enumerate() {
      let position = position as i64;
      sqlx::query!(
        "INSERT INTO album_tracks (album_id, position, track_id) VALUES (?, ?, ?)",
        album_id,
        position,
        track_id
      )
      .execute(&mut *conn)
      .await?;
    }
    Ok(true)
  }
}
