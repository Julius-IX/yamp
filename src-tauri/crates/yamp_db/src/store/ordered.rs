//! The ordered track list of a playlist.
//!
//! A list is stored as rows `(playlist_id, position, track_id)` with positions counting from 0
//! and no gaps. Edits rewrite the whole list instead of shifting positions in place.
//! SQLite checks a primary key per row while an `UPDATE` runs, so `position = position + 1`
//! can hit a row that has not moved yet. Lists are short enough that a rewrite is cheap.

use sqlx::SqliteConnection;

use crate::Result;

pub(crate) mod playlist {
  use super::*;

  pub(crate) async fn ids(conn: &mut SqliteConnection, playlist_id: &str) -> Result<Vec<String>> {
    Ok(
      sqlx::query_scalar!(
        r#"SELECT track_id AS "track_id!" FROM playlist_tracks WHERE playlist_id = ? ORDER BY position"#,
        playlist_id
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
    playlist_id: &str,
    wanted: &[String],
  ) -> Result<bool> {
    if ids(&mut *conn, playlist_id).await? == wanted {
      return Ok(false);
    }
    sqlx::query!(
      "DELETE FROM playlist_tracks WHERE playlist_id = ?",
      playlist_id
    )
    .execute(&mut *conn)
    .await?;
    for (position, track_id) in wanted.iter().enumerate() {
      let position = position as i64;
      sqlx::query!(
        "INSERT INTO playlist_tracks (playlist_id, position, track_id) VALUES (?, ?, ?)",
        playlist_id,
        position,
        track_id
      )
      .execute(&mut *conn)
      .await?;
    }
    Ok(true)
  }
}
