//! What a device tells the others about itself.
//!
//! Each device owns one folder, `devices/<id>`, and is its only writer. It holds the
//! device's change log and a small `meta.json`.

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::Result;

/// The id of this device. It is made once and kept in the database. A copy of the
/// database file would carry the same id, so a copy must never be used as a second device,
/// since every id has to have exactly one writer.
pub(crate) async fn device_id(pool: &SqlitePool) -> Result<String> {
  let found = sqlx::query_scalar!("SELECT value FROM link_meta WHERE key = 'device_id'")
    .fetch_optional(pool)
    .await?;
  if let Some(id) = found {
    return Ok(id);
  }
  let id = Uuid::new_v4().to_string();
  sqlx::query!(
    "INSERT OR IGNORE INTO link_meta (key, value) VALUES ('device_id', ?)",
    id
  )
  .execute(pool)
  .await?;
  // another task may have won the insert
  Ok(
    sqlx::query_scalar!("SELECT value FROM link_meta WHERE key = 'device_id'")
      .fetch_one(pool)
      .await?,
  )
}
