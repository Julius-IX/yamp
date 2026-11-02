use std::{path::Path, str::FromStr};

use sqlx::{
  sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
  SqlitePool,
};

use crate::{migrate, Result};

/// Handle to the library database. Clones share one connection pool.
#[derive(Clone)]
pub struct Db {
  pool: SqlitePool,
}

impl Db {
  /// Opens the database file, creating it and its folder when missing, and brings the
  /// schema up to date.
  pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
    let path = path.as_ref();
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
      tokio::fs::create_dir_all(parent).await?;
    }
    let options = SqliteConnectOptions::new()
      .filename(path)
      .create_if_missing(true)
      // Readers do not block the writer, which matters while a sync writes in the background.
      .journal_mode(SqliteJournalMode::Wal)
      .synchronous(SqliteSynchronous::Normal)
      .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
      .max_connections(4)
      .connect_with(options)
      .await?;
    Self::init(pool).await
  }

  /// A throwaway database that lives as long as the handle. Meant for tests.
  pub async fn memory() -> Result<Self> {
    Self::init(memory_pool().await?).await
  }

  /// An in-memory database migrated only up to migration number `upto`.
  #[cfg(test)]
  pub(crate) async fn memory_at(upto: usize) -> Result<Self> {
    let pool = memory_pool().await?;
    migrate::run_to(&pool, upto).await?;
    Ok(Self { pool })
  }

  async fn init(pool: SqlitePool) -> Result<Self> {
    migrate::run(&pool).await?;
    Ok(Self { pool })
  }

  pub fn pool(&self) -> &SqlitePool {
    &self.pool
  }

  /// Waits for running queries and closes every connection. The WAL file is folded back
  /// into the database on the last close.
  pub async fn close(&self) {
    self.pool.close().await;
  }
}

/// Every connection to `:memory:` is a database of its own, so the pool is pinned to a
/// single connection that is never recycled.
async fn memory_pool() -> Result<SqlitePool> {
  Ok(
    SqlitePoolOptions::new()
      .max_connections(1)
      .min_connections(1)
      .idle_timeout(None)
      .max_lifetime(None)
      .connect_with(SqliteConnectOptions::from_str("sqlite::memory:")?.foreign_keys(true))
      .await?,
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn creates_missing_folders_and_reopens() {
    let dir = std::env::temp_dir().join(format!("yamp-db-open-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("nested").join("library.db");

    let db = Db::open(&path).await.unwrap();
    db.close().await;
    assert!(path.exists());
    Db::open(&path).await.unwrap().close().await;
    let _ = std::fs::remove_dir_all(dir);
  }
}
