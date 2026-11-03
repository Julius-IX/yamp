//! What a device tells the others about itself.
//!
//! Each device owns one folder, `devices/<id>`, and is its only writer. It holds the
//! device's change log and a small `meta.json`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sqlx::{SqliteConnection, SqlitePool};
use uuid::Uuid;

use super::{blocking, Link};
use crate::{clock::now_ms, Error, Result};

pub(crate) const META: &str = "meta.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DeviceMeta {
  pub name: String,
  pub created_ms: i64,
  /// When the device last synced. Devices that were silent for long stop holding back the
  /// cleanup of logs.
  pub last_seen_ms: i64,
  /// Log entries up to this name were deleted. A reader that has not got this far cannot
  /// continue from the log and starts over from a snapshot.
  #[serde(default)]
  pub trimmed_through: Option<String>,
  /// For every other device, the last entry of its log this device has applied. A device
  /// deletes its entries only once everyone has applied them.
  #[serde(default)]
  pub acks: BTreeMap<String, String>,
}

/// A device that shares the library.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
  pub id: String,
  pub name: String,
  pub last_seen_ms: i64,
  pub this_device: bool,
}

/// Same as [`Link::cursors`] on a connection that is already held, since asking the pool
/// for a second one while a transaction is open can wait forever on a small pool.
pub(crate) async fn cursors_on(conn: &mut SqliteConnection) -> Result<BTreeMap<String, String>> {
  let rows = sqlx::query!("SELECT device, cursor FROM link_cursors")
    .fetch_all(conn)
    .await?;
  Ok(rows.into_iter().map(|r| (r.device, r.cursor)).collect())
}

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

impl Link {
  /// Ids of the other devices that have a folder.
  pub(crate) async fn peers(&self) -> Result<Vec<String>> {
    let dir = self.root.clone();
    let mut ids = blocking(move || dir.dir("devices")?.list_dirs()).await?;
    ids.retain(|id| *id != self.device);
    Ok(ids)
  }

  pub(crate) async fn meta_of(&self, device: &str) -> Result<Option<DeviceMeta>> {
    let dir = self.device_dir(device)?;
    blocking(move || dir.get_json_opt::<DeviceMeta>(META)).await
  }

  /// Cursors into the logs of the other devices, as stored after each applied entry.
  pub(crate) async fn cursors(&self) -> Result<BTreeMap<String, String>> {
    let mut conn = self.pool.acquire().await?;
    cursors_on(&mut conn).await
  }

  /// Updates the meta file of this device. Only this device writes it.
  pub(crate) async fn update_meta(
    &self,
    change: impl FnOnce(&mut DeviceMeta) + Send + 'static,
  ) -> Result<()> {
    let dir = self.device_dir(&self.device)?;
    let name = self.name.clone();
    blocking(move || {
      let now = now_ms();
      let mut meta = dir.get_json_opt::<DeviceMeta>(META)?.unwrap_or(DeviceMeta {
        name: name.clone(),
        created_ms: now,
        last_seen_ms: now,
        trimmed_through: None,
        acks: BTreeMap::new(),
      });
      meta.name = name;
      meta.last_seen_ms = now;
      change(&mut meta);
      dir.put_json(META, &meta).map(|_| ())
    })
    .await
  }

  /// Tells the others how far this device has read.
  pub(crate) async fn publish(&self) -> Result<()> {
    let acks = self.cursors().await?;
    self.update_meta(move |m| m.acks = acks).await
  }

  /// Every device that shares the library, this one included.
  pub async fn devices(&self) -> Result<Vec<DeviceInfo>> {
    let mut out = Vec::new();
    for id in self.peers().await? {
      if let Some(meta) = self.meta_of(&id).await? {
        out.push(DeviceInfo {
          id,
          name: meta.name,
          last_seen_ms: meta.last_seen_ms,
          this_device: false,
        });
      }
    }
    out.push(DeviceInfo {
      id: self.device.clone(),
      name: self.name.clone(),
      last_seen_ms: now_ms(),
      this_device: true,
    });
    out.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
    Ok(out)
  }

  /// Removes another device from the shared folder, for example one that was lost. Its
  /// log and snapshot go with it. The device itself, if it still exists, links again with
  /// whatever is in its own library the next time it syncs.
  pub async fn forget_device(&self, id: &str) -> Result<()> {
    if id == self.device {
      return Err(Error::Invalid("a device cannot forget itself".into()));
    }
    let dir = self.device_dir(id)?;
    blocking(move || dir.delete_dir().map(|_| ())).await?;
    sqlx::query!("DELETE FROM link_cursors WHERE device = ?", id)
      .execute(&self.pool)
      .await?;
    Ok(())
  }
}
