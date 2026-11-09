//! Reading entities out of the tables as documents.

use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Serialize;
use serde_json::Value;
use sqlx::SqliteConnection;

use super::{
  doc::{ArtistDoc, Change, CoverDoc, LikeDoc, PlaylistDoc, TrackDoc},
  kind::Kind,
  state,
};
use crate::{store::{is_local_id, ordered}, Result};

fn to_value(doc: &impl Serialize) -> Result<Option<Value>> {
  Ok(Some(serde_json::to_value(doc)?))
}

/// The entity as it is now, or `None` when it does not exist, which is how a deletion is
/// told from a change.
pub(crate) async fn load(
  conn: &mut SqliteConnection,
  kind: Kind,
  key: &str,
) -> Result<Option<Value>> {
  match kind {
    Kind::Track => {
      let row = sqlx::query!(
        r#"SELECT title, artist, album, duration_secs, thumbnail, release_year AS "release_year: u32"
           FROM tracks WHERE id = ?"#,
        key
      )
      .fetch_optional(&mut *conn)
      .await?;
      row.map_or(Ok(None), |r| {
        to_value(&TrackDoc {
          title: r.title,
          artist: r.artist,
          album: r.album,
          duration_secs: r.duration_secs,
          thumbnail: r.thumbnail,
          release_year: r.release_year,
        })
      })
    }
    Kind::Like => {
      let liked = sqlx::query_scalar!("SELECT liked_ms FROM likes WHERE track_id = ?", key)
        .fetch_optional(&mut *conn)
        .await?;
      liked.map_or(Ok(None), |liked_ms| to_value(&LikeDoc { liked_ms }))
    }
    Kind::Artist => {
      let row = sqlx::query!(
        "SELECT name, thumbnail, followed_ms FROM followed_artists WHERE id = ?",
        key
      )
      .fetch_optional(&mut *conn)
      .await?;
      row.map_or(Ok(None), |r| {
        to_value(&ArtistDoc {
          name: r.name,
          thumbnail: r.thumbnail,
          followed_ms: r.followed_ms,
        })
      })
    }
    Kind::Playlist => {
      let row = sqlx::query!(
        "SELECT name, description, created_ms, updated_ms FROM playlists WHERE id = ?",
        key
      )
      .fetch_optional(&mut *conn)
      .await?;
      let Some(r) = row else { return Ok(None) };
      let tracks = ordered::playlist::ids(conn, key).await?;
      to_value(&PlaylistDoc {
        name: r.name,
        description: r.description,
        created_ms: r.created_ms,
        updated_ms: r.updated_ms,
        tracks,
      })
    }
    Kind::Cover => {
      let row = sqlx::query!(
        "SELECT image, updated_ms FROM playlist_covers WHERE playlist_id = ?",
        key
      )
      .fetch_optional(&mut *conn)
      .await?;
      row.map_or(Ok(None), |r| {
        to_value(&CoverDoc { image: STANDARD.encode(r.image), updated_ms: r.updated_ms })
      })
    }
  }
}

/// A change that is ready to be sent.
#[derive(Debug, Clone)]
pub(crate) struct Outgoing {
  pub change: Change,
  /// The time the entity was marked dirty with. Marking it clean afterwards only
  /// succeeds if this is still the time, so an edit made while sending is not lost.
  pub dirty_ts: i64,
}

/// Everything changed here that has not been sent, as changes made by `device`.
///
/// A change is sent with a time later than the version it replaces, even when the clock
/// of this device is behind the one that made that version. Without it an edit that was
/// made after seeing the other version could lose to it on every other device.
pub(crate) async fn outgoing(conn: &mut SqliteConnection, device: &str) -> Result<Vec<Outgoing>> {
  let mut out = Vec::new();
  for dirty in state::dirty(conn).await? {
    let kind = dirty.kind.as_str();
    let known = sqlx::query_scalar!(
      "SELECT ts_ms FROM link_versions WHERE kind = ? AND key = ?",
      kind,
      dirty.key
    )
    .fetch_optional(&mut *conn)
    .await?;
    let ts_ms = known.map_or(dirty.ts_ms, |k| dirty.ts_ms.max(k + 1));
    out.push(Outgoing {
      change: Change {
        kind: kind.into(),
        key: dirty.key.clone(),
        ts_ms,
        device: device.into(),
        doc: load(conn, dirty.kind, &dirty.key).await?,
      },
      dirty_ts: dirty.ts_ms,
    });
  }
  Ok(out)
}

/// The version a snapshot reports for an entity that exists. A change that is still
/// pending counts as made by this device, with the same time `outgoing` would give it.
async fn version_of(
  conn: &mut SqliteConnection,
  kind: Kind,
  key: &str,
  me: &str,
) -> Result<(i64, String)> {
  let kind_name = kind.as_str();
  let dirty = sqlx::query_scalar!(
    "SELECT ts_ms FROM link_dirty WHERE kind = ? AND key = ?",
    kind_name,
    key
  )
  .fetch_optional(&mut *conn)
  .await?;
  let known = sqlx::query!(
    "SELECT ts_ms, device FROM link_versions WHERE kind = ? AND key = ?",
    kind_name,
    key
  )
  .fetch_optional(&mut *conn)
  .await?
  .map(|r| (r.ts_ms, r.device));
  Ok(match (dirty, known) {
    (Some(d), Some((v, _))) => (d.max(v + 1), me.into()),
    (Some(d), None) => (d, me.into()),
    (None, Some(known)) => known,
    (None, None) => (0, me.into()),
  })
}

/// The keys of every entity of a kind that exists.
async fn keys(conn: &mut SqliteConnection, kind: Kind) -> Result<Vec<String>> {
  Ok(match kind {
    Kind::Track => {
      let ids = sqlx::query_scalar!(r#"SELECT id AS "key!" FROM tracks"#)
        .fetch_all(conn)
        .await?;
      // a local track has no file on another device, and the triggers never note it for the log
      ids.into_iter().filter(|id| !is_local_id(id)).collect()
    }
    Kind::Like => {
      sqlx::query_scalar!(r#"SELECT track_id AS "key!" FROM likes"#)
        .fetch_all(conn)
        .await?
    }
    Kind::Artist => {
      sqlx::query_scalar!(r#"SELECT id AS "key!" FROM followed_artists"#)
        .fetch_all(conn)
        .await?
    }
    Kind::Playlist => {
      sqlx::query_scalar!(r#"SELECT id AS "key!" FROM playlists"#)
        .fetch_all(conn)
        .await?
    }
    Kind::Cover => {
      sqlx::query_scalar!(r#"SELECT playlist_id AS "key!" FROM playlist_covers"#)
        .fetch_all(conn)
        .await?
    }
  })
}

/// The whole library as changes, deletions included. A deletion has to be in there, since
/// a device that applies the snapshot may still hold the entity.
pub(crate) async fn snapshot_changes(conn: &mut SqliteConnection, me: &str) -> Result<Vec<Change>> {
  let mut out = Vec::new();
  for kind in Kind::ALL {
    for key in keys(conn, kind).await? {
      let (ts_ms, device) = version_of(conn, kind, &key, me).await?;
      out.push(Change {
        kind: kind.as_str().into(),
        doc: load(conn, kind, &key).await?,
        key,
        ts_ms,
        device,
      });
    }
  }
  let tombstones =
    sqlx::query!("SELECT kind, key, ts_ms, device FROM link_versions WHERE deleted = 1")
      .fetch_all(conn)
      .await?;
  out.extend(tombstones.into_iter().map(|r| Change {
    kind: r.kind,
    key: r.key,
    ts_ms: r.ts_ms,
    device: r.device,
    doc: None,
  }));
  Ok(out)
}
