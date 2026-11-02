//! Reading entities out of the tables as documents.

use serde::Serialize;
use serde_json::Value;
use sqlx::SqliteConnection;

use super::{
  doc::{AlbumDoc, ArtistDoc, Change, LikeDoc, PlaylistDoc, TrackDoc},
  kind::Kind,
  state,
};
use crate::{store::ordered, Result};

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
    Kind::Album => {
      let row = sqlx::query!(
        r#"SELECT title, artist, cover, release_year AS "release_year: u32", added_ms
           FROM albums WHERE id = ?"#,
        key
      )
      .fetch_optional(&mut *conn)
      .await?;
      let Some(r) = row else { return Ok(None) };
      let tracks = ordered::album::ids(conn, key).await?;
      to_value(&AlbumDoc {
        title: r.title,
        artist: r.artist,
        cover: r.cover,
        release_year: r.release_year,
        added_ms: r.added_ms,
        tracks,
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
