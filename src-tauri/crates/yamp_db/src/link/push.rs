use super::{
  blocking,
  doc::{Batch, Change},
  export::{self, Outgoing},
  Link,
};
use crate::Result;

/// A log entry is one request, so it stays well under the object limit of the backends.
const BATCH_BYTES: usize = 1024 * 1024;

impl Link {
  /// Appends everything changed on this device since the last push to its log and returns
  /// how many entities that was. Nothing is written when nothing changed.
  pub async fn push(&self) -> Result<usize> {
    let out = {
      let mut conn = self.pool.acquire().await?;
      export::outgoing(&mut conn, &self.device).await?
    };
    let total = out.len();
    let log = self.log_of(&self.device)?;

    for batch in split(out)? {
      let body = Batch {
        changes: batch.iter().map(|o| o.change.clone()).collect(),
      };
      let log = log.clone();
      let name = blocking(move || log.append(&body)).await?;
      self.mark_sent(&batch, &name).await?;
    }
    if total > 0 {
      log::info!("pushed {total} changes");
    }
    Ok(total)
  }

  /// Marks a sent batch as done. A batch is marked before the next one is written, so a
  /// failure part way sends only what is left. Sending a change twice is harmless anyway,
  /// the second copy is not newer than the first.
  async fn mark_sent(&self, batch: &[Outgoing], entry: &str) -> Result<()> {
    let mut tx = self.pool.begin().await?;
    for o in batch {
      let c = &o.change;
      let deleted = i64::from(c.doc.is_none());
      sqlx::query!(
        "INSERT INTO link_versions (kind, key, ts_ms, device, deleted) VALUES (?, ?, ?, ?, ?)
         ON CONFLICT (kind, key) DO UPDATE SET
           ts_ms = excluded.ts_ms, device = excluded.device, deleted = excluded.deleted",
        c.kind,
        c.key,
        c.ts_ms,
        c.device,
        deleted
      )
      .execute(&mut *tx)
      .await?;
      // only if it was not edited again meanwhile, that edit goes out with the next push
      sqlx::query!(
        "DELETE FROM link_dirty WHERE kind = ? AND key = ? AND ts_ms = ?",
        c.kind,
        c.key,
        o.dirty_ts
      )
      .execute(&mut *tx)
      .await?;
    }
    sqlx::query!(
      "INSERT INTO link_meta (key, value) VALUES ('own_cursor', ?)
       ON CONFLICT (key) DO UPDATE SET value = excluded.value",
      entry
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
  }
}

fn size_of(change: &Change) -> Result<usize> {
  Ok(serde_json::to_vec(change)?.len())
}

fn split(out: Vec<Outgoing>) -> Result<Vec<Vec<Outgoing>>> {
  let mut batches: Vec<Vec<Outgoing>> = Vec::new();
  let mut used = 0;
  for o in out {
    let size = size_of(&o.change)?;
    match batches.last_mut() {
      Some(last) if used + size <= BATCH_BYTES => {
        last.push(o);
        used += size;
      }
      _ => {
        batches.push(vec![o]);
        used = size;
      }
    }
  }
  Ok(batches)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn outgoing(key: &str, pad: usize) -> Outgoing {
    Outgoing {
      change: Change {
        kind: "track".into(),
        key: key.into(),
        ts_ms: 1,
        device: "d".into(),
        doc: Some(serde_json::json!({ "pad": "x".repeat(pad) })),
      },
      dirty_ts: 1,
    }
  }

  #[test]
  fn batches_stay_under_the_limit() {
    let out: Vec<_> = (0..5)
      .map(|i| outgoing(&i.to_string(), 400 * 1024))
      .collect();
    let sizes: Vec<usize> = split(out).unwrap().iter().map(Vec::len).collect();
    assert_eq!(sizes, [2, 2, 1]);
    assert!(split(Vec::new()).unwrap().is_empty());
  }

  #[test]
  fn one_big_change_gets_a_batch_of_its_own() {
    let out = vec![outgoing("a", 10), outgoing("b", 2 * BATCH_BYTES)];
    assert_eq!(split(out).unwrap().len(), 2);
  }
}
