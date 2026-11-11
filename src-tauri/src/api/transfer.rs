//! Moving the library in and out as CSV. Reading, matching and writing the files is in
//! `yamp-transfer`, which only deals with text. This is the part that asks the provider and writes to
//! the library.
use super::{
  fail,
  wire::{track_to_db, tracks_to_db},
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use ts_types::TsType;
use yamp_transfer::{Candidate, Entry, Failure, Kind, Reason};
use yamp_db::Db;
use yt_api::{AlbumPage, ItemKind, Track, YtApi};

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
