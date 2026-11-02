//! Snapshots and the cleanup of logs.
//!
//! A log only grows, so entries that every device has read are deleted. That makes a
//! log useless for a device that has not read them, such as a new one or one that was
//! away for months. Such a device starts from a snapshot, the whole library of some
//! device at one moment, and then reads each log from where the snapshot ends.
//!
//! Every device keeps one snapshot under its own id, which replaces the last one. The
//! blob is written before `snapshot.json` that describes it. A reader trusting the
//! description of an older blob only reads more log entries than needed, which is harmless
//! since applying a change twice does nothing.
//!
//! Before deleting entries a device writes `trimmed_through` to its meta file. A reader
//! that sees it does not read past a gap without noticing. The other order would leave a
//! moment where entries are gone and nothing says so.

use cloud_kit::Blobs;

use super::{
  apply::apply,
  blocking,
  device::{cursors_on, DeviceMeta},
  doc::{Snapshot, SnapshotInfo},
  export,
  pull::has_gap,
  Link,
};
use crate::{clock::now_ms, Error, Result};

const SNAPSHOTS: &str = "snapshots";
const INFO: &str = "snapshot.json";

/// A device that has not synced for this long no longer holds back the cleanup of logs.
/// It starts from a snapshot when it comes back.
const INACTIVE_MS: i64 = 60 * 24 * 60 * 60 * 1000;

/// How far this device may delete its own log, `None` when it may not delete anything.
///
/// Entries go once every active device has read them, and never past `own`, the newest
/// entry that is part of this device's snapshot. A device that never reported a position
/// blocks the cleanup, since nothing is known about what it has read.
pub(crate) fn trim_limit(me: &str, peers: &[DeviceMeta], own: &str, now: i64) -> Option<String> {
  let mut limit = own.to_string();
  for peer in peers.iter().filter(|p| now - p.last_seen_ms <= INACTIVE_MS) {
    let ack = peer.acks.get(me)?;
    if *ack < limit {
      limit = ack.clone();
    }
  }
  Some(limit)
}

impl Link {
  /// Peers whose log has a gap for this device.
  pub(crate) async fn gaps(&self) -> Result<Vec<String>> {
    let cursors = self.cursors().await?;
    let mut out = Vec::new();
    for peer in self.peers().await? {
      let trimmed = self.meta_of(&peer).await?.and_then(|m| m.trimmed_through);
      if has_gap(trimmed.as_deref(), cursors.get(&peer).map(String::as_str)) {
        out.push(peer);
      }
    }
    Ok(out)
  }

  fn blobs(&self) -> Result<Blobs> {
    Ok(Blobs::new(self.root.dir(SNAPSHOTS)?))
  }

  /// Publishes the whole library of this device as its snapshot.
  ///
  /// The snapshot is held in memory while it is built and uploaded. That is fine for the
  /// metadata of a library, which is a few megabytes for tens of thousands of tracks.
  pub async fn write_snapshot(&self) -> Result<()> {
    // one transaction, so the covered positions match the library that is exported
    let mut tx = self.pool.begin().await?;
    let mut covers = cursors_on(&mut tx).await?;
    let own = sqlx::query_scalar!("SELECT value FROM link_meta WHERE key = 'own_cursor'")
      .fetch_optional(&mut *tx)
      .await?;
    covers.extend(own.map(|c| (self.device.clone(), c)));
    let changes = export::snapshot_changes(&mut tx, &self.device).await?;
    drop(tx);

    let bytes = serde_json::to_vec(&Snapshot { changes })?;
    let size = bytes.len();
    let info = SnapshotInfo {
      created_ms: now_ms(),
      covers,
    };
    let (blobs, dir, id) = (
      self.blobs()?,
      self.device_dir(&self.device)?,
      self.device.clone(),
    );
    blocking(move || {
      blobs.put(&id, &bytes)?;
      dir.put_json(INFO, &info).map(|_| ())
    })
    .await?;
    log::info!("wrote snapshot of {size} bytes");
    Ok(())
  }

  /// Starts from the newest snapshot that reaches past every cleanup. Afterwards the
  /// logs are read from the positions the snapshot covers.
  pub(crate) async fn restore(&self) -> Result<()> {
    let mut best: Option<(String, SnapshotInfo)> = None;
    let metas = self.peer_metas().await?;
    for (peer, _) in &metas {
      let dir = self.device_dir(peer)?;
      let Some(info) = blocking(move || dir.get_json_opt::<SnapshotInfo>(INFO)).await? else {
        continue;
      };
      let usable = metas.iter().all(|(device, meta)| {
        !has_gap(
          meta.trimmed_through.as_deref(),
          info.covers.get(device).map(String::as_str),
        )
      });
      if usable
        && best
          .as_ref()
          .is_none_or(|(_, b)| info.created_ms > b.created_ms)
      {
        best = Some((peer.clone(), info));
      }
    }
    let (from, info) = best.ok_or(Error::NoSnapshot)?;

    let (blobs, name) = (self.blobs()?, from.clone());
    let bytes = blocking(move || blobs.get(&name)).await?;
    let snapshot: Snapshot = serde_json::from_slice(&bytes)?;
    let cursors: Vec<(&str, &str)> = info
      .covers
      .iter()
      .filter(|(device, _)| **device != self.device)
      .map(|(d, c)| (d.as_str(), c.as_str()))
      .collect();
    let applied = apply(&self.pool, &self.device, &snapshot.changes, &cursors).await?;
    log::info!("restored {applied} changes from the snapshot of {from}");
    Ok(())
  }

  async fn peer_metas(&self) -> Result<Vec<(String, DeviceMeta)>> {
    let mut out = Vec::new();
    for peer in self.peers().await? {
      out.extend(self.meta_of(&peer).await?.map(|m| (peer, m)));
    }
    Ok(out)
  }

  /// Syncs, publishes a snapshot and deletes the log entries that every active device has
  /// read. Meant to run now and then, for example once a week, and not at every sync.
  pub async fn compact(&self) -> Result<()> {
    self.sync().await?;
    self.write_snapshot().await?;

    let own = sqlx::query_scalar!("SELECT value FROM link_meta WHERE key = 'own_cursor'")
      .fetch_optional(&self.pool)
      .await?;
    let Some(own) = own else { return Ok(()) };
    let peers: Vec<DeviceMeta> = self
      .peer_metas()
      .await?
      .into_iter()
      .map(|(_, m)| m)
      .collect();
    let Some(limit) = trim_limit(&self.device, &peers, &own, now_ms()) else {
      return Ok(());
    };

    let marked = limit.clone();
    self
      .update_meta(move |m| {
        if m.trimmed_through.as_ref().is_none_or(|t| *t < marked) {
          m.trimmed_through = Some(marked);
        }
      })
      .await?;
    let log = self.log_of(&self.device)?;
    let removed = blocking(move || log.trim_through(&limit)).await?;
    log::info!("removed {removed} log entries");
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn peer(last_seen_ms: i64, ack: Option<&str>) -> DeviceMeta {
    DeviceMeta {
      name: "p".into(),
      created_ms: 0,
      last_seen_ms,
      trimmed_through: None,
      acks: ack
        .map(|a| ("me".to_string(), a.to_string()))
        .into_iter()
        .collect(),
    }
  }

  #[test]
  fn trims_to_the_slowest_active_reader() {
    let now = 10 * INACTIVE_MS;
    let peers = [peer(now, Some("c")), peer(now, Some("b"))];
    assert_eq!(trim_limit("me", &peers, "e", now).as_deref(), Some("b"));
    // never past what the snapshot covers
    assert_eq!(trim_limit("me", &peers, "a", now).as_deref(), Some("a"));
  }

  #[test]
  fn a_device_that_never_reported_blocks_the_cleanup() {
    let now = 10 * INACTIVE_MS;
    let peers = [peer(now, Some("c")), peer(now, None)];
    assert_eq!(trim_limit("me", &peers, "e", now), None);
  }

  #[test]
  fn silent_devices_stop_counting() {
    let now = 10 * INACTIVE_MS;
    let peers = [peer(now, Some("d")), peer(now - INACTIVE_MS - 1, None)];
    assert_eq!(trim_limit("me", &peers, "e", now).as_deref(), Some("d"));
    assert_eq!(trim_limit("me", &[], "e", now).as_deref(), Some("e"));
  }
}
