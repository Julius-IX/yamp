//! Moving the library in and out as CSV. Reading, matching and writing the files is in
//! `yamp-transfer`, which only deals with text. This is the part that asks the provider and writes to
//! the library.
use super::{
  fail,
  wire::{track_to_db, tracks_to_db},
};
use std::io::Write;
use tauri_plugin_fs::{FsExt, OpenOptions};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use ts_types::TsType;
use yamp_transfer::{Candidate, Entry, Failure, Kind, Reason};
use yamp_db::Db;
use tauri_plugin_dialog::DialogExt;
use yt_api::{AlbumPage, ItemKind, Track, YtApi};

/// Rows of likes read per query when the library is exported.
const PAGE: u32 = 500;

/// Emitted while an import looks songs up, with a [`Progress`] as payload.
const PROGRESS_EVENT: &str = "transfer:progress";

/// Lookups that run at once. More would be faster, but the provider answers a burst of searches
/// with errors, and those would show up as failed entries that were never really missing.
const CONCURRENCY: usize = 6;

#[derive(Clone, Serialize)]
struct Progress {
  done: usize,
  total: usize,
}

/// What the import of one file did.
#[derive(Serialize, TsType)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
  /// The service the file looks like it came from.
  pub source: String,
  /// Rows that were looked up. Songs of one album count once.
  pub total: usize,
  pub liked: usize,
  /// Playlists that were created. One is made per name in the file, also when the same file is
  /// imported twice, since a playlist that exists may have been edited since.
  pub playlists: usize,
  pub albums: usize,
  pub failures: Vec<FailedEntry>,
  /// The failures as a file that can be saved and imported again.
  pub failed_csv: String,
}

#[derive(Serialize, TsType)]
#[serde(rename_all = "camelCase")]
pub struct FailedEntry {
  /// Position in the file, the header of a CSV being 1. In the JSON of Spotify it is the position
  /// in the list the entry is in.
  pub row: usize,
  /// Like, playlist or album.
  pub kind: String,
  /// The playlist, or the album for a song of one. None for a like.
  pub collection: Option<String>,
  /// The album name for an album.
  pub title: String,
  pub artist: String,
  pub reason: String,
}

impl From<&Failure> for FailedEntry {
  fn from(f: &Failure) -> Self {
    let e = &f.entry;
    let title = if e.kind == Kind::Album { e.album.clone().unwrap_or_default() } else { e.title.clone() };
    Self {
      row: e.row,
      kind: e.kind.as_str().into(),
      collection: e.collection.clone(),
      title,
      artist: e.artist.clone(),
      reason: f.reason.as_str().into(),
    }
  }
}

enum Found {
  Song(Track),
  Album(AlbumPage),
  Missed(Reason),
}

/// Reads a CSV file or the JSON of the account data of Spotify and adds what the provider has to
/// the library. `name` is the name of the file without its extension, which names the playlist
/// when the rows do not. An entry that cannot be
/// placed does not stop the import, it ends up in the report.
#[tauri_groups::command(transfer)]
pub async fn import_file(
  app: AppHandle,
  db: State<'_, Db>,
  name: String,
  text: String,
) -> Result<ImportReport, String> {
  let parsed = yamp_transfer::parse(&text, &name).map_err(|e| e.to_string())?;
  let total = parsed.entries.len() + parsed.rejected.len();
  let wanted = parsed.entries.len();
  let mut failed = parsed.rejected;

  let mut found = Vec::with_capacity(wanted);
  for chunk in parsed.entries.chunks(CONCURRENCY) {
    let tasks: Vec<_> =
      chunk.iter().map(|e| tauri::async_runtime::spawn(look_up(app.clone(), e.clone()))).collect();
    for task in tasks {
      found.push(task.await.unwrap_or(Found::Missed(Reason::LookupFailed)));
    }
    let _ = app.emit(PROGRESS_EVENT, Progress { done: found.len(), total: wanted });
  }

  let (mut liked, mut albums) = (0, 0);
  // kept in the order of the file, a HashMap would shuffle the playlists
  let mut playlists: Vec<(String, Vec<(Entry, Track)>)> = Vec::new();
  for (entry, result) in parsed.entries.into_iter().zip(found) {
    match result {
      Found::Missed(reason) => failed.push(Failure { entry, reason }),
      Found::Song(track) if entry.kind == Kind::Like => match db.likes().like(&track_to_db(track)).await {
        Ok(()) => liked += 1,
        Err(e) => {
          fail(e);
          failed.push(Failure { entry, reason: Reason::SaveFailed });
        }
      },
      Found::Song(track) => {
        let name = entry.collection.clone().unwrap_or_default();
        match playlists.iter_mut().find(|(n, _)| *n == name) {
          Some((_, songs)) => songs.push((entry, track)),
          None => playlists.push((name, vec![(entry, track)])),
        }
      }
      Found::Album(page) => {
        // same as saving the album from its page, which is also what makes saving it twice harmless
        let description = [Some(page.artist.clone()), page.year.map(|y| y.to_string())]
          .into_iter()
          .flatten()
          .filter(|p| !p.is_empty())
          .collect::<Vec<_>>()
          .join(" · ");
        let cover = page.thumbnail.clone();
        let tracks = page
          .tracks
          .into_iter()
          .map(|mut t| {
            if t.thumbnail.is_none() {
              t.thumbnail = cover.clone();
            }
            t
          })
          .collect();
        let description = (!description.is_empty()).then_some(description.as_str());
        let saved = db.playlists().save_as(&page.id, &page.title, description, &tracks_to_db(tracks)).await;
        match saved {
          Ok(_) => albums += 1,
          Err(e) => {
            fail(e);
            failed.push(Failure { entry, reason: Reason::SaveFailed });
          }
        }
      }
    }
  }

  let mut created = 0;
  for (name, songs) in playlists {
    let tracks: Vec<Track> = songs.iter().map(|(_, t)| t.clone()).collect();
    let saved = save_playlist(&db, &name, tracks).await;
    match saved {
      Ok(()) => created += 1,
      Err(e) => {
        fail(e);
        failed.extend(songs.into_iter().map(|(entry, _)| Failure { entry, reason: Reason::SaveFailed }));
      }
    }
  }

  failed.sort_by_key(|f| f.entry.row);
  Ok(ImportReport {
    source: parsed.source.name().into(),
    total,
    liked,
    playlists: created,
    albums,
    failures: failed.iter().map(FailedEntry::from).collect(),
    failed_csv: yamp_transfer::failures_csv(&failed),
  })
}

async fn save_playlist(db: &Db, name: &str, tracks: Vec<Track>) -> yamp_db::Result<()> {
  let playlist = db.playlists().create(name, None).await?;
  db.playlists().add_tracks(&playlist.id, &tracks_to_db(tracks)).await
}

/// Asks the provider for the song or album and keeps the result only when it is close enough,
/// see `yamp_transfer::pick`.
async fn look_up(app: AppHandle, entry: Entry) -> Found {
  let api = app.state::<YtApi>();
  let query = yamp_transfer::query(&entry);
  if entry.kind == Kind::Album {
    let Ok(items) = api.search_items(&query, ItemKind::Album, Some(5)).await else {
      return Found::Missed(Reason::LookupFailed);
    };
    // the subtitle of an album is where the provider lists the artist
    let candidates: Vec<Candidate> = items
      .iter()
      .map(|i| Candidate { title: i.title.clone(), artist: i.subtitle.clone().unwrap_or_default(), duration_secs: None })
      .collect();
    let Some(at) = yamp_transfer::pick(&entry, &candidates) else { return Found::Missed(Reason::NoMatch) };
    return match api.album(&items[at].id).await {
      Ok(page) => Found::Album(page),
      Err(_) => Found::Missed(Reason::LookupFailed),
    };
  }
  let Ok(tracks) = api.search(&query, Some(10)).await else { return Found::Missed(Reason::LookupFailed) };
  let candidates: Vec<Candidate> = tracks
    .iter()
    .map(|t| Candidate {
      title: t.title.clone(),
      artist: t.artist.clone(),
      duration_secs: t.duration_secs.map(|d| d.round() as u32),
    })
    .collect();
  match yamp_transfer::pick(&entry, &candidates) {
    Some(at) => Found::Song(tracks.into_iter().nth(at).expect("pick returns an index of the candidates")),
    None => Found::Missed(Reason::NoMatch),
  }
}

/// The likes, the playlists and the saved albums as one file in the columns of `yamp-transfer`.
/// Nothing in it ties the library to the provider, so any other service or tool can read it.
/// Downloads and local folders are left out, they are files of this device and not part of
/// the library.
#[tauri_groups::command(transfer)]
pub async fn export_csv(db: State<'_, Db>) -> Result<String, String> {
  let mut entries = Vec::new();

  let mut offset = 0;
  loop {
    let page = db.likes().list(PAGE, offset).await.map_err(fail)?;
    let more = page.len() == PAGE as usize;
    offset += PAGE;
    entries.extend(page.iter().map(|t| entry(Kind::Like, None, None, t)));
    if !more {
      break;
    }
  }

  for playlist in db.playlists().list().await.map_err(fail)? {
    let tracks = db.playlists().tracks(&playlist.id).await.map_err(fail)?;
    if is_uuid(&playlist.id) {
      entries.extend(tracks.iter().map(|t| entry(Kind::Playlist, Some(&playlist.name), None, t)));
    } else {
      // the description of a saved album starts with its artist, which is the one artist all
      // of its songs share, so the import looks the album up once
      let artist = playlist.description.as_deref().and_then(|d| d.split(" · ").next());
      entries.extend(tracks.iter().map(|t| entry(Kind::Album, Some(&playlist.name), artist, t)));
    }
  }
  Ok(yamp_transfer::library_csv(&entries))
}

fn entry(kind: Kind, collection: Option<&str>, artist: Option<&str>, t: &yamp_db::Track) -> Entry {
  Entry {
    row: 0,
    kind,
    collection: collection.map(Into::into),
    title: t.title.clone(),
    artist: artist.unwrap_or(&t.artist).into(),
    // the saved album that a song is listed under is the album it is looked up by
    album: if kind == Kind::Album { collection.map(Into::into) } else { t.album.clone() },
    duration_secs: t.duration_secs.map(|d| d.round() as u32),
    year: t.release_year,
  }
}

/// Playlists made in the app have a random id, saved albums have the id of the provider. This
/// is the same rule as `isSavedAlbum` in the frontend.
fn is_uuid(id: &str) -> bool {
  id.len() == 36
    && id.char_indices().all(|(i, c)| if matches!(i, 8 | 13 | 18 | 23) { c == '-' } else { c.is_ascii_hexdigit() })
}

/// Asks where to save and writes the file there. Returns the path, or None when the dialog was
/// closed without choosing. The dialog is the only thing that picks the path, so the frontend
/// cannot write anywhere on its own.
#[tauri_groups::command(transfer)]
pub async fn save_csv(app: AppHandle, file_name: String, csv: String) -> Result<Option<String>, String> {
  // the dialog blocks until it is answered
  tauri::async_runtime::spawn_blocking(move || -> Result<Option<String>, String> {
    let Some(chosen) = app.dialog().file().add_filter("CSV", &["csv"]).set_file_name(file_name).blocking_save_file()
    else {
      return Ok(None);
    };
    let mut opts = OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    let mut file = app.fs().open(chosen.clone(), opts).map_err(|e| e.to_string())?;
    file.write_all(csv.as_bytes()).map_err(|e| e.to_string())?;
    Ok(Some(chosen.to_string()))
  })
  .await
  .map_err(|e| e.to_string())?
}
