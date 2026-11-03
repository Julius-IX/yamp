//! Writing changes from other devices into the tables.
//!
//! Every entity has one winner, the change with the latest time. Equal times are decided
//! by the device id, so all devices pick the same one. A deletion is a change like any
//! other and its tombstone is kept in `link_versions`, so an older copy of the entity
//! that arrives late does not bring it back.

use serde::de::DeserializeOwned;
use serde_json::Value;
use sqlx::{SqliteConnection, SqlitePool};

use super::{
  doc::{AlbumDoc, ArtistDoc, Change, LikeDoc, PlaylistDoc, TrackDoc},
  kind::Kind,
  state,
};
use crate::{store::ordered, Result};

/// Writes `changes` in one transaction and returns how many took effect. The others were
/// older than what is known, belong to a kind this version does not have or were not
/// readable, and are skipped without an error so one bad entry cannot block a sync.
///
/// `cursors` are log positions as `(device, entry name)`, stored in the same transaction.
/// After a crash a log entry is either fully applied and skipped, or not applied at all.
/// A cursor never moves back.
pub(crate) async fn apply(
  pool: &SqlitePool,
  me: &str,
  changes: &[Change],
  cursors: &[(&str, &str)],
) -> Result<usize> {
  let mut tx = pool.begin().await?;
  state::set_applying(&mut tx, true).await?;
  let mut applied = 0;
  for change in changes {
    if apply_one(&mut tx, me, change).await? {
      applied += 1;
    }
  }
  for (device, name) in cursors {
    sqlx::query!(
      "INSERT INTO link_cursors (device, cursor) VALUES (?, ?)
       ON CONFLICT (device) DO UPDATE SET cursor = MAX(cursor, excluded.cursor)",
      device,
      name
    )
    .execute(&mut *tx)
    .await?;
  }
  state::set_applying(&mut tx, false).await?;
  tx.commit().await?;
  Ok(applied)
}

async fn apply_one(conn: &mut SqliteConnection, me: &str, change: &Change) -> Result<bool> {
  let Some(kind) = Kind::parse(&change.kind) else {
    log::debug!("skipping a change of unknown kind {:?}", change.kind);
    return Ok(false);
  };
  if !is_newer(conn, kind, change, me).await? {
    return Ok(false);
  }
  match &change.doc {
    Some(doc) => {
      if !put(conn, kind, &change.key, doc).await? {
        log::warn!("skipping unreadable {} {}", change.kind, change.key);
        return Ok(false);
      }
    }
    None => delete(conn, kind, &change.key).await?,
  }
  let deleted = i64::from(change.doc.is_none());
  sqlx::query!(
    "INSERT INTO link_versions (kind, key, ts_ms, device, deleted) VALUES (?, ?, ?, ?, ?)
     ON CONFLICT (kind, key) DO UPDATE SET
       ts_ms = excluded.ts_ms, device = excluded.device, deleted = excluded.deleted",
    change.kind,
    change.key,
    change.ts_ms,
    change.device,
    deleted
  )
  .execute(&mut *conn)
  .await?;
  // a pending local change was older than this one and is replaced by it
  sqlx::query!(
    "DELETE FROM link_dirty WHERE kind = ? AND key = ?",
    change.kind,
    change.key
  )
  .execute(conn)
  .await?;
  Ok(true)
}

/// Whether the change beats the newest version this device knows, which is either a
/// version from a sync or its own change that is still waiting to be sent.
async fn is_newer(
  conn: &mut SqliteConnection,
  kind: Kind,
  change: &Change,
  me: &str,
) -> Result<bool> {
  let kind = kind.as_str();
  let mut newest: Option<(i64, String)> = sqlx::query_scalar!(
    "SELECT ts_ms FROM link_dirty WHERE kind = ? AND key = ?",
    kind,
    change.key
  )
  .fetch_optional(&mut *conn)
  .await?
  .map(|ts| (ts, me.to_string()));
  let version = sqlx::query!(
    "SELECT ts_ms, device FROM link_versions WHERE kind = ? AND key = ?",
    kind,
    change.key
  )
  .fetch_optional(&mut *conn)
  .await?;
  if let Some(v) = version {
    let known = (v.ts_ms, v.device);
    if newest.as_ref().is_none_or(|n| known > *n) {
      newest = Some(known);
    }
  }
  Ok(newest.is_none_or(|n| (change.ts_ms, change.device.clone()) > n))
}

fn read<T: DeserializeOwned>(doc: &Value) -> Option<T> {
  serde_json::from_value(doc.clone()).ok()
}

async fn delete(conn: &mut SqliteConnection, kind: Kind, key: &str) -> Result<()> {
  match kind {
    Kind::Track => {
      sqlx::query!("DELETE FROM tracks WHERE id = ?", key)
        .execute(conn)
        .await?
    }
    Kind::Like => {
      sqlx::query!("DELETE FROM likes WHERE track_id = ?", key)
        .execute(conn)
        .await?
    }
    Kind::Artist => {
      sqlx::query!("DELETE FROM followed_artists WHERE id = ?", key)
        .execute(conn)
        .await?
    }
    Kind::Album => {
      sqlx::query!("DELETE FROM albums WHERE id = ?", key)
        .execute(conn)
        .await?
    }
    Kind::Playlist => {
      sqlx::query!("DELETE FROM playlists WHERE id = ?", key)
        .execute(conn)
        .await?
    }
  };
  Ok(())
}

/// Replaces the entity with the document. Returns false when the document is not readable.
async fn put(conn: &mut SqliteConnection, kind: Kind, key: &str, doc: &Value) -> Result<bool> {
  match kind {
    Kind::Track => {
      let Some(d) = read::<TrackDoc>(doc) else {
        return Ok(false);
      };
      sqlx::query!(
        "INSERT INTO tracks (id, title, artist, album, duration_secs, thumbnail, release_year)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT (id) DO UPDATE SET
           title = excluded.title, artist = excluded.artist, album = excluded.album,
           duration_secs = excluded.duration_secs, thumbnail = excluded.thumbnail,
           release_year = excluded.release_year",
        key,
        d.title,
        d.artist,
        d.album,
        d.duration_secs,
        d.thumbnail,
        d.release_year
      )
      .execute(conn)
      .await?;
    }
    Kind::Like => {
      let Some(d) = read::<LikeDoc>(doc) else {
        return Ok(false);
      };
      sqlx::query!(
        "INSERT INTO likes (track_id, liked_ms) VALUES (?, ?)
         ON CONFLICT (track_id) DO UPDATE SET liked_ms = excluded.liked_ms",
        key,
        d.liked_ms
      )
      .execute(conn)
      .await?;
    }
    Kind::Artist => {
      let Some(d) = read::<ArtistDoc>(doc) else {
        return Ok(false);
      };
      sqlx::query!(
        "INSERT INTO followed_artists (id, name, thumbnail, followed_ms) VALUES (?, ?, ?, ?)
         ON CONFLICT (id) DO UPDATE SET
           name = excluded.name, thumbnail = excluded.thumbnail, followed_ms = excluded.followed_ms",
        key,
        d.name,
        d.thumbnail,
        d.followed_ms
      )
      .execute(conn)
      .await?;
    }
    Kind::Album => {
      let Some(d) = read::<AlbumDoc>(doc) else {
        return Ok(false);
      };
      sqlx::query!(
        "INSERT INTO albums (id, title, artist, cover, release_year, added_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT (id) DO UPDATE SET
           title = excluded.title, artist = excluded.artist, cover = excluded.cover,
           release_year = excluded.release_year, added_ms = excluded.added_ms",
        key,
        d.title,
        d.artist,
        d.cover,
        d.release_year,
        d.added_ms
      )
      .execute(&mut *conn)
      .await?;
      ordered::album::replace(conn, key, &d.tracks).await?;
    }
    Kind::Playlist => {
      let Some(d) = read::<PlaylistDoc>(doc) else {
        return Ok(false);
      };
      sqlx::query!(
        "INSERT INTO playlists (id, name, description, created_ms, updated_ms)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (id) DO UPDATE SET
           name = excluded.name, description = excluded.description,
           created_ms = excluded.created_ms, updated_ms = excluded.updated_ms",
        key,
        d.name,
        d.description,
        d.created_ms,
        d.updated_ms
      )
      .execute(&mut *conn)
      .await?;
      ordered::playlist::replace(conn, key, &d.tracks).await?;
    }
  }
  Ok(true)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{link::export, store::track::tests::track, Album, Artist, Db};
  use serde_json::json;

  fn change(kind: &str, key: &str, ts_ms: i64, device: &str, doc: Option<Value>) -> Change {
    Change {
      kind: kind.into(),
      key: key.into(),
      ts_ms,
      device: device.into(),
      doc,
    }
  }

  fn playlist(name: &str, tracks: &[&str]) -> Value {
    json!({ "name": name, "description": null, "created_ms": 1, "updated_ms": 1, "tracks": tracks })
  }

  async fn name_of(db: &Db, id: &str) -> Option<String> {
    db.playlists().get(id).await.unwrap().map(|p| p.name)
  }

  /// Sends everything pending on `from` to `to`, the way a push and a pull would.
  async fn ship(from: &Db, from_id: &str, to: &Db, to_id: &str) {
    let mut conn = from.pool().acquire().await.unwrap();
    let out = export::outgoing(&mut conn, from_id).await.unwrap();
    drop(conn);
    let changes: Vec<Change> = out.into_iter().map(|o| o.change).collect();
    apply(to.pool(), to_id, &changes, &[]).await.unwrap();
  }

  async fn pending(db: &Db) -> usize {
    let mut conn = db.pool().acquire().await.unwrap();
    state::dirty(&mut conn).await.unwrap().len()
  }

  #[tokio::test]
  async fn every_kind_arrives_intact() {
    let (a, b) = (Db::memory().await.unwrap(), Db::memory().await.unwrap());
    a.likes().like(&track("t1", "One")).await.unwrap();
    a.artists()
      .follow(&Artist {
        id: "ar".into(),
        name: "Artist".into(),
        thumbnail: Some("a.jpg".into()),
      })
      .await
      .unwrap();
    let album = Album {
      id: "al".into(),
      title: "Album".into(),
      artist: "Artist".into(),
      cover: None,
      release_year: Some(2020),
    };
    a.albums()
      .save(&album, &[track("t3", "Three"), track("t2", "Two")])
      .await
      .unwrap();
    let p = a.playlists().create("Mix", Some("d")).await.unwrap();
    a.playlists()
      .add_tracks(
        &p.id,
        &[track("t2", "Two"), track("t1", "One"), track("t2", "Two")],
      )
      .await
      .unwrap();

    ship(&a, "a", &b, "b").await;

    assert_eq!(
      b.likes().ids().await.unwrap(),
      a.likes().ids().await.unwrap()
    );
    assert_eq!(
      b.artists().list().await.unwrap(),
      a.artists().list().await.unwrap()
    );
    assert_eq!(
      b.albums().list().await.unwrap(),
      a.albums().list().await.unwrap()
    );
    assert_eq!(
      b.albums().tracks("al").await.unwrap(),
      a.albums().tracks("al").await.unwrap()
    );
    assert_eq!(
      b.playlists().get(&p.id).await.unwrap(),
      a.playlists().get(&p.id).await.unwrap()
    );
    assert_eq!(
      b.playlists().tracks(&p.id).await.unwrap(),
      a.playlists().tracks(&p.id).await.unwrap()
    );
    // what arrived is not sent back
    assert_eq!(pending(&b).await, 0);
  }

  #[tokio::test]
  async fn the_latest_change_wins() {
    let db = Db::memory().await.unwrap();
    let put = |ts, device, name| change("playlist", "p", ts, device, Some(playlist(name, &[])));
    apply(db.pool(), "me", &[put(100, "x", "first")], &[])
      .await
      .unwrap();

    // older loses, newer wins
    assert_eq!(
      apply(db.pool(), "me", &[put(50, "y", "older")], &[])
        .await
        .unwrap(),
      0
    );
    assert_eq!(name_of(&db, "p").await.as_deref(), Some("first"));
    apply(db.pool(), "me", &[put(200, "x", "second")], &[])
      .await
      .unwrap();
    assert_eq!(name_of(&db, "p").await.as_deref(), Some("second"));

    // equal times go to the greater device id, on every device alike
    apply(db.pool(), "me", &[put(200, "a", "loses")], &[])
      .await
      .unwrap();
    assert_eq!(name_of(&db, "p").await.as_deref(), Some("second"));
    apply(db.pool(), "me", &[put(200, "z", "wins")], &[])
      .await
      .unwrap();
    assert_eq!(name_of(&db, "p").await.as_deref(), Some("wins"));
  }

  #[tokio::test]
  async fn a_deletion_is_not_undone_by_an_older_copy() {
    let db = Db::memory().await.unwrap();
    let put = |ts| change("playlist", "p", ts, "x", Some(playlist("Mix", &["a"])));
    apply(db.pool(), "me", &[put(100)], &[]).await.unwrap();
    apply(
      db.pool(),
      "me",
      &[change("playlist", "p", 200, "x", None)],
      &[],
    )
    .await
    .unwrap();
    assert_eq!(name_of(&db, "p").await, None);

    apply(db.pool(), "me", &[put(150)], &[]).await.unwrap();
    assert_eq!(name_of(&db, "p").await, None);
    apply(db.pool(), "me", &[put(300)], &[]).await.unwrap();
    assert_eq!(name_of(&db, "p").await.as_deref(), Some("Mix"));
    assert_eq!(db.playlists().tracks("p").await.unwrap().len(), 0);
  }

  #[tokio::test]
  async fn a_pending_local_change_competes_with_an_incoming_one() {
    let db = Db::memory().await.unwrap();
    let p = db.playlists().create("Local", None).await.unwrap();
    assert_eq!(pending(&db).await, 1);

    // from a long time ago, the local edit is newer
    let old = change("playlist", &p.id, 1, "x", Some(playlist("Remote", &[])));
    assert_eq!(apply(db.pool(), "me", &[old], &[]).await.unwrap(), 0);
    assert_eq!(name_of(&db, &p.id).await.as_deref(), Some("Local"));
    assert_eq!(pending(&db).await, 1);

    // from the future, the remote change is newer and replaces the pending one
    let new = change(
      "playlist",
      &p.id,
      i64::MAX,
      "x",
      Some(playlist("Remote", &[])),
    );
    assert_eq!(apply(db.pool(), "me", &[new], &[]).await.unwrap(), 1);
    assert_eq!(name_of(&db, &p.id).await.as_deref(), Some("Remote"));
    assert_eq!(pending(&db).await, 0);
  }

  #[tokio::test]
  async fn a_change_that_is_not_readable_does_not_block_the_rest() {
    let db = Db::memory().await.unwrap();
    let changes = [
      change("podcast", "x", 1, "x", Some(json!({}))),
      change("playlist", "bad", 1, "x", Some(json!({ "name": 5 }))),
      change("playlist", "good", 1, "x", Some(playlist("Fine", &[]))),
    ];

    assert_eq!(apply(db.pool(), "me", &changes, &[]).await.unwrap(), 1);
    assert_eq!(name_of(&db, "good").await.as_deref(), Some("Fine"));
    assert_eq!(name_of(&db, "bad").await, None);
  }

  #[tokio::test]
  async fn the_cursor_moves_with_the_changes() {
    let db = Db::memory().await.unwrap();
    let c = change("playlist", "p", 1, "x", Some(playlist("Mix", &[])));
    apply(db.pool(), "me", &[c], &[("x", "entry-1")])
      .await
      .unwrap();
    apply(db.pool(), "me", &[], &[("x", "entry-2")])
      .await
      .unwrap();

    let cursor: String = sqlx::query_scalar("SELECT cursor FROM link_cursors WHERE device = 'x'")
      .fetch_one(db.pool())
      .await
      .unwrap();
    assert_eq!(cursor, "entry-2");
  }

  #[tokio::test]
  async fn a_late_local_edit_is_sent_with_a_time_after_the_version_it_saw() {
    let db = Db::memory().await.unwrap();
    // a version from a device whose clock is far ahead
    let ahead = change(
      "playlist",
      "p",
      i64::MAX / 2,
      "x",
      Some(playlist("Remote", &[])),
    );
    apply(db.pool(), "me", &[ahead], &[]).await.unwrap();

    db.playlists()
      .update("p", "Edited here", None)
      .await
      .unwrap();
    let mut conn = db.pool().acquire().await.unwrap();
    let out = export::outgoing(&mut conn, "me").await.unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].change.ts_ms, i64::MAX / 2 + 1);
    assert!(out[0].dirty_ts < out[0].change.ts_ms);
  }
}
