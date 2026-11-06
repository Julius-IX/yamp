use std::{
  collections::BTreeMap,
  fs,
  io::Write,
  path::PathBuf,
  sync::{Arc, Mutex, PoisonError},
};

use cloud_sync::Provider;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// What is kept of a sign in. The access token is short lived and never written down.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Saved {
  pub refresh_token: String,
  pub account: Option<String>,
}

/// The refresh tokens of all services in one file, readable by the owner only where the
/// platform has such a thing. It is a file of its own and not part of the settings, since the
/// settings are handed to the webview and a token has no business there.
#[derive(Clone)]
pub struct TokenStore {
  path: Arc<PathBuf>,
  // one writer at a time, a token refresh and a sign out can overlap
  lock: Arc<Mutex<()>>,
}

type File = BTreeMap<String, Saved>;

fn key(provider: Provider) -> String {
  // the persisted names of Provider are stable, see its docs
  serde_json::to_value(provider)
    .ok()
    .and_then(|v| v.as_str().map(str::to_owned))
    .unwrap_or_default()
}

impl TokenStore {
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self { path: Arc::new(path.into()), lock: Arc::default() }
  }

  fn read(&self) -> File {
    fs::read(&*self.path)
      .ok()
      .and_then(|b| serde_json::from_slice(&b).ok())
      .unwrap_or_default()
  }

  fn write(&self, file: &File) -> Result<()> {
    let tmp = self.path.with_extension("tmp");
    let mut out = fs::OpenOptions::new();
    out.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut out, 0o600);
    let json = serde_json::to_vec(file).map_err(|e| Error::Protocol(e.to_string()))?;
    out.open(&tmp)?.write_all(&json)?;
    // a rename keeps the old file whole if the write above was cut short
    fs::rename(&tmp, &*self.path)?;
    Ok(())
  }

  pub fn load(&self, provider: Provider) -> Option<Saved> {
    let _guard = self.lock.lock().unwrap_or_else(PoisonError::into_inner);
    self.read().remove(&key(provider))
  }

  pub fn save(&self, provider: Provider, saved: Saved) -> Result<()> {
    let _guard = self.lock.lock().unwrap_or_else(PoisonError::into_inner);
    let mut file = self.read();
    file.insert(key(provider), saved);
    self.write(&file)
  }

  pub fn remove(&self, provider: Provider) -> Result<()> {
    let _guard = self.lock.lock().unwrap_or_else(PoisonError::into_inner);
    let mut file = self.read();
    if file.remove(&key(provider)).is_some() {
      self.write(&file)?;
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn tokens_are_kept_per_service_and_removed() {
    let dir = std::env::temp_dir().join(format!("cloud-auth-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let store = TokenStore::new(dir.join("tokens.json"));
    let saved = |t: &str| Saved { refresh_token: t.into(), account: None };

    store.save(Provider::GoogleDrive, saved("g")).unwrap();
    store.save(Provider::OneDrive, saved("o")).unwrap();
    assert_eq!(store.load(Provider::GoogleDrive).unwrap().refresh_token, "g");
    store.remove(Provider::GoogleDrive).unwrap();
    assert!(store.load(Provider::GoogleDrive).is_none());
    assert_eq!(store.load(Provider::OneDrive).unwrap().refresh_token, "o");
    fs::remove_dir_all(dir).unwrap();
  }
}
