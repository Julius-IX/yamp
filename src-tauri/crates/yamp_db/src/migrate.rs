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
