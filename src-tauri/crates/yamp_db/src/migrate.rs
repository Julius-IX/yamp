//! Schema versioning.
//!
//! The files in `src-tauri/migrations` are embedded at compile time and applied in order.
//! sqlx records every applied file with a checksum in `_sqlx_migrations`, so a file that
//! was edited after it ran, or a database written by a newer app, is refused.
//!
//! Files are never edited after a release. A change to the schema is a new file at the end.
//! They live at the workspace root, where `sqlx-cli` expects them, so the same folder
//! builds the database that the query macros check against.

use sqlx::{migrate::Migrator, SqlitePool};

use crate::Result;

static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");

pub(crate) async fn run(pool: &SqlitePool) -> Result<()> {
  MIGRATOR.run(pool).await?;
  Ok(())
}

/// Applies only the first `count` migrations. Tests use it to build a database as an older
/// version of the app left it.
#[cfg(test)]
pub(crate) async fn run_to(pool: &SqlitePool, count: usize) -> Result<()> {
  let partial = Migrator {
    migrations: std::borrow::Cow::Owned(MIGRATOR.migrations[..count].to_vec()),
    ..Migrator::DEFAULT
  };
  partial.run(pool).await?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{Db, Error};
  use sqlx::migrate::MigrateError;

  #[tokio::test]
  async fn is_idempotent() {
    let db = Db::memory().await.unwrap();
    run(db.pool()).await.unwrap();
    run(db.pool()).await.unwrap();
    let applied: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
      .fetch_one(db.pool())
      .await
      .unwrap();
    assert_eq!(applied as usize, MIGRATOR.migrations.len());
  }

  #[tokio::test]
  async fn saved_albums_become_playlists() {
    // a library as the app left it while albums were still their own thing
    let db = Db::memory_at(7).await.unwrap();
    for sql in [
      "INSERT INTO tracks (id, title, artist, thumbnail) VALUES ('a', 'A', 'X', NULL), ('b', 'B', 'X', 'b.jpg')",
      "INSERT INTO albums (id, title, artist, cover, release_year, added_ms)
       VALUES ('alb', 'Album', 'X', 'cover.jpg', 2001, 5), ('bare', 'Bare', '', NULL, NULL, 6)",
      "INSERT INTO album_tracks (album_id, position, track_id) VALUES ('alb', 0, 'b'), ('alb', 1, 'a')",
      "INSERT INTO link_versions (kind, key, ts_ms, device) VALUES ('album', 'alb', 1, 'd')",
    ] {
      sqlx::query(sql).execute(db.pool()).await.unwrap();
    }
    // the triggers of the link migration noted the album as changed
    let noted: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM link_dirty WHERE kind = 'album'")
      .fetch_one(db.pool())
      .await
      .unwrap();
    assert_eq!(noted, 2);

    run(db.pool()).await.unwrap();

    let album = db.playlists().get("alb").await.unwrap().unwrap();
    assert_eq!(album.name, "Album");
    assert_eq!(album.description.as_deref(), Some("X · 2001"));
    assert_eq!((album.created_ms, album.track_count), (5, 2));
    let order: Vec<_> = db.playlists().tracks("alb").await.unwrap();
    assert_eq!(order.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(), ["b", "a"]);
    // only a track without a thumbnail takes the cover
    let thumbs: Vec<_> = order.iter().map(|t| t.thumbnail.as_deref()).collect();
    assert_eq!(thumbs, [Some("b.jpg"), Some("cover.jpg")]);

    let bare = db.playlists().get("bare").await.unwrap().unwrap();
    assert_eq!((bare.description, bare.track_count), (None, 0));

    let left: i64 = sqlx::query_scalar(
      "SELECT (SELECT COUNT(*) FROM link_dirty WHERE kind = 'album')
            + (SELECT COUNT(*) FROM link_versions WHERE kind = 'album')
            + (SELECT COUNT(*) FROM sqlite_master WHERE name IN ('albums', 'album_tracks'))",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();
    assert_eq!(left, 0);
  }

  #[tokio::test]
  async fn refuses_a_database_from_a_newer_app() {
    let db = Db::memory().await.unwrap();
    sqlx::query(
      "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
       VALUES (999, 'from the future', 1, x'00', 0)",
    )
    .execute(db.pool())
    .await
    .unwrap();
    assert!(matches!(
      run(db.pool()).await,
      Err(Error::Migrate(MigrateError::VersionMissing(999)))
    ));
  }
}
