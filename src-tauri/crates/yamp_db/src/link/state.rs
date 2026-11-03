//! Reading and writing the bookkeeping tables of `0007_link.sql`.

use sqlx::SqliteConnection;

use super::kind::Kind;
use crate::Result;

/// An entity that changed on this device and has not been sent yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Dirty {
  pub kind: Kind,
  pub key: String,
  pub ts_ms: i64,
}

/// Oldest first, so a failed send resumes where it stopped.
pub(crate) async fn dirty(conn: &mut SqliteConnection) -> Result<Vec<Dirty>> {
  let rows = sqlx::query!("SELECT kind, key, ts_ms FROM link_dirty ORDER BY ts_ms, kind, key")
    .fetch_all(conn)
    .await?;
  // a kind this version does not know cannot be in this table, only the triggers write it
  Ok(
    rows
      .into_iter()
      .filter_map(|r| {
        Some(Dirty {
          kind: Kind::parse(&r.kind)?,
          key: r.key,
          ts_ms: r.ts_ms,
        })
      })
      .collect(),
  )
}

/// While set, changes are not noted as local ones. Only meant to be called inside the
/// transaction that writes a change from another device, so a failure resets it with the
/// rollback and other connections never see it set.
pub(crate) async fn set_applying(conn: &mut SqliteConnection, on: bool) -> Result<()> {
  let on = i64::from(on);
  sqlx::query!("UPDATE link_flag SET applying = ?", on)
    .execute(conn)
    .await?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{store::track::tests::track, Album, Artist, Db};

  async fn pending(db: &Db) -> Vec<(String, String)> {
    let mut conn = db.pool().acquire().await.unwrap();
    let mut out: Vec<_> = dirty(&mut conn)
      .await
      .unwrap()
      .into_iter()
      .map(|d| (d.kind.as_str().to_string(), d.key))
      .collect();
    out.sort();
    out
  }

  fn pair(kind: &str, key: &str) -> (String, String) {
    (kind.into(), key.into())
  }

  #[tokio::test]
  async fn every_shared_entity_is_noted() {
    let db = Db::memory().await.unwrap();
    db.likes().like(&track("t1", "One")).await.unwrap();
    db.artists()
      .follow(&Artist {
        id: "ar".into(),
        name: "Artist".into(),
        thumbnail: None,
      })
      .await
      .unwrap();
    db.albums()
      .save(
        &Album {
          id: "al".into(),
          title: "Album".into(),
          artist: "Artist".into(),
          cover: None,
          release_year: None,
        },
        &[track("t2", "Two")],
      )
      .await
      .unwrap();
    let p = db.playlists().create("Mix", None).await.unwrap();
    db.playlists()
      .add_tracks(&p.id, &[track("t3", "Three")])
      .await
      .unwrap();

    let mut expected = vec![
      pair("track", "t1"),
      pair("track", "t2"),
      pair("track", "t3"),
      pair("like", "t1"),
      pair("artist", "ar"),
      pair("album", "al"),
      pair("playlist", &p.id),
    ];
    expected.sort();
    assert_eq!(pending(&db).await, expected);
  }

  #[tokio::test]
  async fn a_list_edit_marks_its_owner() {
    let db = Db::memory().await.unwrap();
    let p = db.playlists().create("Mix", None).await.unwrap();
    db.playlists()
      .add_tracks(&p.id, &[track("a", "A"), track("b", "B")])
      .await
      .unwrap();
    sqlx::query("DELETE FROM link_dirty WHERE kind = 'playlist'")
      .execute(db.pool())
      .await
      .unwrap();

    db.playlists().move_track(&p.id, 0, 1).await.unwrap();
    assert!(pending(&db).await.contains(&pair("playlist", &p.id)));
  }

  #[tokio::test]
  async fn deleting_marks_the_entity_too() {
    let db = Db::memory().await.unwrap();
    let p = db.playlists().create("Mix", None).await.unwrap();
    sqlx::query("DELETE FROM link_dirty")
      .execute(db.pool())
      .await
      .unwrap();

    db.playlists().delete(&p.id).await.unwrap();
    assert_eq!(pending(&db).await, [pair("playlist", &p.id)]);
  }

  #[tokio::test]
  async fn saving_unchanged_data_leaves_no_trace() {
    let db = Db::memory().await.unwrap();
    db.tracks().save(&track("a", "A")).await.unwrap();
    let before: i64 = sqlx::query_scalar("SELECT ts_ms FROM link_dirty")
      .fetch_one(db.pool())
      .await
      .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(3));

    db.tracks().save(&track("a", "A")).await.unwrap();
    let after: i64 = sqlx::query_scalar("SELECT ts_ms FROM link_dirty")
      .fetch_one(db.pool())
      .await
      .unwrap();
    assert_eq!(before, after);
  }

  #[tokio::test]
  async fn downloads_are_not_shared() {
    let db = Db::memory().await.unwrap();
    db.downloads()
      .record(&track("a", "A"), "m4a", 1)
      .await
      .unwrap();
    // only the track metadata is shared
    assert_eq!(pending(&db).await, [pair("track", "a")]);
  }

  #[tokio::test]
  async fn changes_from_other_devices_are_not_echoed() {
    let db = Db::memory().await.unwrap();
    let mut conn = db.pool().acquire().await.unwrap();
    set_applying(&mut conn, true).await.unwrap();
    sqlx::query("INSERT INTO tracks (id, title, artist) VALUES ('r', 'Remote', 'X')")
      .execute(&mut *conn)
      .await
      .unwrap();
    set_applying(&mut conn, false).await.unwrap();
    assert!(dirty(&mut conn).await.unwrap().is_empty());

    sqlx::query("INSERT INTO tracks (id, title, artist) VALUES ('l', 'Local', 'X')")
      .execute(&mut *conn)
      .await
      .unwrap();
    assert_eq!(dirty(&mut conn).await.unwrap().len(), 1);
  }

  #[tokio::test]
  async fn existing_data_is_marked_when_the_migration_runs() {
    // a library as the app left it before devices could be linked
    let db = Db::memory_at(6).await.unwrap();
    sqlx::query("INSERT INTO tracks (id, title, artist) VALUES ('old', 'Old', 'X')")
      .execute(db.pool())
      .await
      .unwrap();
    sqlx::query("INSERT INTO likes (track_id, liked_ms) VALUES ('old', 1)")
      .execute(db.pool())
      .await
      .unwrap();

    crate::migrate::run(db.pool()).await.unwrap();
    assert_eq!(
      pending(&db).await,
      [pair("like", "old"), pair("track", "old")]
    );
  }
}
