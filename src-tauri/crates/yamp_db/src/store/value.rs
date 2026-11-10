//! Small key value pairs that are shared between devices.
//!
//! The store knows nothing about what a value means. The synced settings and the listening
//! statistics both live here, told apart by the prefix of their keys. A day of statistics of one
//! device is one key that only that device writes, so two devices never change the same entity.

use sqlx::SqlitePool;

use crate::{Db, Result, clock::now_ms};

impl Db {
  pub fn values(&self) -> Values {
    Values { pool: self.pool().clone() }
  }
}

#[derive(Clone)]
pub struct Values {
  pool: SqlitePool,
}

impl Values {
  /// Writes the value. Writing what is already there changes nothing, so it is not sent again.
  pub async fn set(&self, key: &str, value: &str) -> Result<()> {
    let now = now_ms();
    sqlx::query!(
      "INSERT INTO synced_values (key, value, updated_ms) VALUES (?1, ?2, ?3)
       ON CONFLICT (key) DO UPDATE SET value = excluded.value, updated_ms = excluded.updated_ms
       WHERE value IS NOT excluded.value",
      key,
      value,
      now
    )
    .execute(&self.pool)
    .await?;
    Ok(())
  }

  /// Every pair whose key starts with `prefix`, in key order. The prefix is compared as plain
  /// text, so `%` and `_` in it mean themselves.
  pub async fn list(&self, prefix: &str) -> Result<Vec<(String, String)>> {
    let rows = sqlx::query!(
      r#"SELECT key AS "key!", value AS "value!" FROM synced_values
         WHERE substr(key, 1, length(?)) = ? ORDER BY key"#,
      prefix,
      prefix
    )
    .fetch_all(&self.pool)
    .await?;
    Ok(rows.into_iter().map(|r| (r.key, r.value)).collect())
  }

  /// Returns how many pairs were removed.
  pub async fn remove_prefix(&self, prefix: &str) -> Result<u64> {
    let done = sqlx::query!("DELETE FROM synced_values WHERE substr(key, 1, length(?)) = ?", prefix, prefix)
      .execute(&self.pool)
      .await?;
    Ok(done.rows_affected())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  async fn noted(db: &Db) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM link_dirty WHERE kind = 'value'")
      .fetch_one(db.pool())
      .await
      .unwrap()
  }

  #[tokio::test]
  async fn lists_by_plain_prefix() {
    let db = Db::memory().await.unwrap();
    let values = db.values();
    values.set("setting:theme", "\"ocean\"").await.unwrap();
    values.set("setting:mode", "\"dark\"").await.unwrap();
    values.set("stats:2026-01-01:a", "{}").await.unwrap();
    values.set("settingX", "1").await.unwrap();

    let found = values.list("setting:").await.unwrap();
    let keys: Vec<_> = found.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(keys, ["setting:mode", "setting:theme"]);

    assert_eq!(values.remove_prefix("setting:").await.unwrap(), 2);
    assert_eq!(values.list("").await.unwrap().len(), 2);
  }

  #[tokio::test]
  async fn the_same_value_is_not_noted_again() {
    let db = Db::memory().await.unwrap();
    let values = db.values();
    values.set("setting:theme", "1").await.unwrap();
    sqlx::query("DELETE FROM link_dirty").execute(db.pool()).await.unwrap();

    values.set("setting:theme", "1").await.unwrap();
    assert_eq!(noted(&db).await, 0);
    values.set("setting:theme", "2").await.unwrap();
    assert_eq!(noted(&db).await, 1);
  }
}
