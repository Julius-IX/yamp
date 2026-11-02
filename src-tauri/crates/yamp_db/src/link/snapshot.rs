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
}
