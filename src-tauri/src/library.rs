//! Library state for the commands. The database and the download folder are opened once at
//! startup and handed to Tauri as managed state, so a command asks only for the part it uses.
use std::{
  error::Error,
  path::{Path, PathBuf},
  sync::{
    Arc, PoisonError, RwLock,
    atomic::{AtomicBool, Ordering},
  },
};

use cloud_auth::TokenStore;
use cloud_sync::{CloudStore, Provider};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Listener, Manager};
use tauri_settings::Settings;
use yamp_db::{Db, Link, MediaStore};
use yt_api::{DownloadJob, JobState};

const DB_FILE: &str = "library.db";

/// Setting that remembers where the library is linked to, so the link is restored at the next start.
pub const SYNC_KEY: &str = "sync";

/// File of the sign in tokens, see `cloud_auth::TokenStore`.
const TOKENS_FILE: &str = "cloud_tokens.json";

/// Event with a track id as payload, sent once a finished download is part of the library.
pub const DOWNLOADED_EVENT: &str = "library:downloaded";

/// Opens the library and registers it as managed state. Needs to run inside `setup`.
pub fn init(app: &AppHandle) -> Result<(), Box<dyn Error>> {
  let data_dir = app.path().app_data_dir()?;
  // The folder sits below the one `yt-api` downloads into, so the asset protocol scope for
  // `$APPDATA/music` covers it. It is a folder of its own because a repair deletes every file
  // that no record names, which would include a download that is still running.
  let media_dir = data_dir.join("music").join("library");

  let (db, media) = tauri::async_runtime::block_on(async {
    let db = Db::open(data_dir.join(DB_FILE)).await?;
    let media = MediaStore::open(&db, media_dir).await?;
    Ok::<_, yamp_db::Error>((db, media))
  })?;

  // Records and files drift apart when the OS clears files or the app dies during an import.
  // Nothing is importing yet at this point, which a repair requires.
  match tauri::async_runtime::block_on(media.repair()) {
    Ok(r) if r.missing_files.is_empty() && r.orphan_files.is_empty() => {}
    Ok(r) => log::info!(
      "download repair dropped {} records and {} files",
      r.missing_files.len(),
      r.orphan_files.len()
    ),
    Err(e) => log::warn!("download repair failed: {e}"),
  }

  adopt_finished_downloads(app, db.clone(), media.clone());
  app.manage(SyncState::default());
  app.manage(TokenStore::new(data_dir.join(TOKENS_FILE)));
  app.manage(db);
  app.manage(media);
  restore_link(app);
  log::info!("library ready");
  Ok(())
}

/// The queue of `yt-api` leaves a finished file where it was downloaded and knows only an id.
/// The file is moved into the library folder and recorded here, with the track metadata that
/// `downloads.enqueue` saved before it queued the job.
fn adopt_finished_downloads(app: &AppHandle, db: Db, media: MediaStore) {
  let emitter = app.clone();
  app.listen(yt_api::DOWNLOAD_EVENT, move |event| {
    let Ok(job) = serde_json::from_str::<DownloadJob>(event.payload()) else {
      return;
    };
    let Some(file) = job.file.filter(|_| job.state == JobState::Done) else {
      return;
    };
    let (db, media, emitter) = (db.clone(), media.clone(), emitter.clone());
    tauri::async_runtime::spawn(async move {
      match adopt(&db, &media, &job.id, &file.path).await {
        Ok(true) => {
          let _ = emitter.emit(DOWNLOADED_EVENT, &job.id);
        }
        Ok(false) => {}
        Err(e) => log::error!("could not add the download of {} to the library: {e}", job.id),
      }
    });
  });
}

/// Returns whether the file was taken into the library.
async fn adopt(db: &Db, media: &MediaStore, id: &str, path: &Path) -> yamp_db::Result<bool> {
  let Some(track) = db.tracks().get(id).await? else {
    log::warn!("finished download {id} has no track in the library, the file stays where it is");
    return Ok(false);
  };
  media.import(&track, path).await?;
  Ok(true)
}

/// Links the library again to what the last run was linked to. It runs in the background since a
/// cloud account has to be reached for it, and a start without a network must not wait for that.
fn restore_link(app: &AppHandle) {
  let Some(config) = app.state::<Settings>().get::<SyncConfig>(SYNC_KEY) else { return };
  let app = app.clone();
  tauri::async_runtime::spawn(async move {
    let (db, tokens, sync) = (app.state::<Db>(), app.state::<TokenStore>(), app.state::<SyncState>());
    match connect(&db, &config, &tokens).await {
      Ok(link) => sync.set(Some(Connected { link, config })),
      Err(e) => log::warn!("the linked library is not available: {e}"),
    }
  });
}

/// Where the library is linked to and under which name this device shows up there.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncConfig {
  /// The folder of a link to a plain folder. Empty for a cloud account.
  #[serde(default)]
  pub folder: PathBuf,
  /// The cloud service of the link, none for a plain folder.
  #[serde(default)]
  pub provider: Option<Provider>,
  pub device_name: String,
}

/// A linked folder together with what is needed to link it again at the next start.
#[derive(Clone)]
pub struct Connected {
  pub link: Link,
  pub config: SyncConfig,
}

/// Links the library to where the config says. A cloud account needs the tokens of an earlier
/// sign in, a folder is for something else that keeps it equal between devices.
pub async fn connect(db: &Db, config: &SyncConfig, tokens: &TokenStore) -> Result<Link, String> {
  let store: Arc<dyn CloudStore> = match config.provider {
    Some(provider) => {
      let session = cloud_auth::Session::from_store(provider, tokens.clone()).map_err(|e| e.to_string())?;
      cloud_store(provider, session)?
    }
    None => {
      if config.folder.as_os_str().is_empty() {
        return Err("a folder is needed".into());
      }
      Arc::new(cloud_sync::fs::LocalDirStore::new(config.folder.clone()).map_err(|e| e.to_string())?)
    }
  };
  link(db, &config.device_name, store).await
}

/// The store of a cloud service, signed in through `session`.
pub fn cloud_store(provider: Provider, session: cloud_auth::Session) -> Result<Arc<dyn CloudStore>, String> {
  let settings = cloud_sync::Settings::default().with_tokens(Arc::new(session));
  let store = cloud_sync::open(provider, settings).map_err(|e| e.to_string())?;
  Ok(Arc::from(store))
}

pub async fn link(db: &Db, device_name: &str, store: Arc<dyn CloudStore>) -> Result<Link, String> {
  let name = device_name.trim();
  if name.is_empty() {
    return Err("a device name is needed".into());
  }
  Link::open(db, store, name).await.map_err(|e| e.to_string())
}

/// The link to the cloud folder. Empty until a folder is connected.
#[derive(Default)]
pub struct SyncState {
  current: RwLock<Option<Connected>>,
  busy: AtomicBool,
  /// Raised to abandon a sign in that waits for the browser. Shared with the thread that waits.
  cancel_sign_in: Arc<AtomicBool>,
}

impl SyncState {
  pub fn get(&self) -> Option<Connected> {
    // a panic while holding the lock must not make every later read and write panic
    self.current.read().unwrap_or_else(PoisonError::into_inner).clone()
  }

  pub fn set(&self, next: Option<Connected>) {
    *self.current.write().unwrap_or_else(PoisonError::into_inner) = next;
  }

  /// Hands out the flag of a new sign in, lowered.
  pub fn new_sign_in(&self) -> Arc<AtomicBool> {
    self.cancel_sign_in.store(false, Ordering::Release);
    self.cancel_sign_in.clone()
  }

  pub fn cancel_sign_in(&self) {
    self.cancel_sign_in.store(true, Ordering::Release);
  }

  /// Reserves the link for one long operation. Two syncs at once would both append to the log of
  /// this device, and that log has to have a single writer. The second caller is turned away
  /// instead of queued, since a sync that waits behind another one has nothing left to do.
  pub fn begin(&self) -> Result<Busy<'_>, String> {
    if self.busy.swap(true, Ordering::AcqRel) {
      return Err("a sync is already running".into());
    }
    Ok(Busy(&self.busy))
  }
}

/// Frees the link again when dropped.
pub struct Busy<'a>(&'a AtomicBool);

impl Drop for Busy<'_> {
  fn drop(&mut self) {
    self.0.store(false, Ordering::Release);
  }
}
