//! Library state for the commands. The database and the download folder are opened once at
//! startup and handed to Tauri as managed state, so a command asks only for the part it uses.
use std::{
  error::Error,
  path::{Path, PathBuf},
  sync::{
    Arc, PoisonError, RwLock,
    atomic::{AtomicBool, Ordering},
  },
  time::Duration,
};

use cloud_auth::TokenStore;
use cloud_sync::{CloudStore, Provider};
use file_server::FileServer;
use folder_watch::FolderWatch;
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

/// Event without payload, sent when the local songs changed because a folder was removed or a scan
/// found new or deleted files.
pub const LOCAL_EVENT: &str = "library:local";

/// Opens the library and registers it as managed state. Needs to run inside `setup`.
pub fn init(app: &AppHandle) -> Result<(), Box<dyn Error>> {
  let data_dir = app.path().app_data_dir()?;
  // The folder sits below the one `yt-api` downloads into, so the asset protocol scope for
  // `$APPDATA/music` covers it. It is a folder of its own because a repair deletes every file
  // that no record names, which would include a download that is still running.
  let media_dir = data_dir.join("music").join("downloads");
  // an earlier version called the folder `library`, which says nothing about what is in it
  let old_dir = data_dir.join("music").join("library");
  if old_dir.is_dir() && !media_dir.exists() {
    if let Err(e) = std::fs::rename(&old_dir, &media_dir) {
      log::warn!("could not rename the download folder: {e}");
    }
  }

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

  let watch = start_watch(app);
  // The asset protocol only serves what its scope names, and the folders of the user are not
  // below the data folder. Playing a song before the first scan has finished needs them already.
  for dir in tauri::async_runtime::block_on(db.locals().dirs()).unwrap_or_default() {
    allow_dir(app, Path::new(&dir));
    if let Some(watch) = &watch {
      if let Err(e) = watch.watch(Path::new(&dir)) {
        log::warn!("could not watch {dir}: {e}");
      }
    }
  }
  app.manage(Watch(watch));
  app.manage(Serve(start_server(&db, &media)));

  adopt_finished_downloads(app, db.clone(), media.clone());
  app.manage(SyncState::default());
  app.manage(TokenStore::new(data_dir.join(TOKENS_FILE)));
  app.manage(db);
  app.manage(media);
  scan_all(app);
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

/// Reports changes below the added folders, so a file that is copied in shows up without a restart.
/// `None` when the system cannot watch, the scans at the start and at focus still cover that.
pub struct Watch(pub Option<FolderWatch>);

/// Serves audio files to the webview, on desktop. `None` on a phone, where the asset protocol
/// plays them and Android refuses plain HTTP requests to a local port.
pub struct Serve(pub Option<FileServer>);

fn start_watch(app: &AppHandle) -> Option<FolderWatch> {
  let handle = app.clone();
  // a copy of an album is over after a moment of quiet, which is when it is worth a scan
  let watch = FolderWatch::new(Duration::from_secs(2), move |dir| {
    if let Some(dir) = dir.to_str() {
      scan_dirs(&handle, vec![dir.to_owned()]);
    }
  });
  watch.map_err(|e| log::warn!("folders cannot be watched: {e}")).ok()
}

fn start_server(db: &Db, media: &MediaStore) -> Option<FileServer> {
  if cfg!(any(target_os = "android", target_os = "ios")) {
    return None;
  }
  let (db, media) = (db.clone(), media.clone());
  let server = FileServer::start(move |id| tauri::async_runtime::block_on(audio_path(&db, &media, id)).ok().flatten());
  server.map_err(|e| log::warn!("the file server did not start: {e}")).ok()
}

/// Where the audio of a track is on this device, a download or a file of an added folder.
pub async fn audio_path(db: &Db, media: &MediaStore, id: &str) -> yamp_db::Result<Option<PathBuf>> {
  match media.path(id).await? {
    Some(path) => Ok(Some(path)),
    None => db.locals().path(id).await,
  }
}

/// Where the covers of local songs are cached. It is below the folder that the asset protocol
/// serves, so the webview can load them.
pub fn cover_dir(app: &AppHandle) -> PathBuf {
  app.path().app_data_dir().unwrap_or_default().join("music").join("covers")
}

/// Starts or stops reporting a folder. Failing to watch is not an error for the caller, the scan
/// at the start and at focus still find what changed.
pub fn watch_dir(app: &AppHandle, dir: &Path, on: bool) {
  let Some(watch) = &app.state::<Watch>().0 else { return };
  if on {
    if let Err(e) = watch.watch(dir) {
      log::warn!("could not watch {}: {e}", dir.display());
    }
  } else {
    watch.unwatch(dir);
  }
}

/// Lets the asset protocol serve the audio below a folder that the user added.
pub fn allow_dir(app: &AppHandle, dir: &Path) {
  if let Err(e) = app.asset_protocol_scope().allow_directory(dir, true) {
    log::warn!("could not allow {} for playback: {e}", dir.display());
  }
}

/// Scans every added folder in the background, since a large collection takes a while and the
/// start of the app must not wait for it.
pub fn scan_all(app: &AppHandle) {
  let app = app.clone();
  tauri::async_runtime::spawn(async move {
    match app.state::<Db>().locals().dirs().await {
      Ok(dirs) => scan(&app, dirs).await,
      Err(e) => log::warn!("could not list the local folders: {e}"),
    }
  });
}

/// Scans the given folders in the background.
pub fn scan_dirs(app: &AppHandle, dirs: Vec<String>) {
  let app = app.clone();
  tauri::async_runtime::spawn(async move { scan(&app, dirs).await });
}

/// Scans run one after another. A change that arrives during a scan waits for it, and then finds
/// what the scan did not see.
static SCAN_TURN: tauri::async_runtime::Mutex<()> = tauri::async_runtime::Mutex::const_new(());

async fn scan(app: &AppHandle, dirs: Vec<String>) {
  let _turn = SCAN_TURN.lock().await;
  let db = app.state::<Db>();
  let covers = cover_dir(app);
  let mut changed = false;
  for dir in dirs {
    match db.locals().scan(&dir, &covers).await {
      Ok(r) if r.unreachable => log::warn!("local folder {dir} is not reachable, its songs stay"),
      Ok(r) => {
        changed |= r.added + r.removed > 0;
        log::debug!("scanned {dir}: {} new or changed, {} gone", r.added, r.removed);
      }
      Err(e) => log::warn!("scan of {dir} failed: {e}"),
    }
  }
  if let Err(e) = db.locals().prune_covers(&covers).await {
    log::warn!("could not prune the covers: {e}");
  }
  // a scan that found nothing new is silent, otherwise every return to the app would reload lists
  if changed {
    let _ = app.emit(LOCAL_EVENT, ());
  }
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
