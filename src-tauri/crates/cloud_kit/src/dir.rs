use std::{collections::BTreeSet, sync::Arc};

use cloud_sync::{validate_key, CloudStore, Error as StoreError, ObjectMeta};

use crate::{Error, Result};

/// A directory of the store. Directories have no existence of their own, they are
/// the shared key prefix of the objects below, so nothing needs creating or removing.
///
/// Names are relative to the directory and may nest (`a/b`). Every object returned
/// by a method here carries such a relative name in `key`. Listings read everything
/// below the directory in one go, so trees should stay shallow and big groups of
/// files should live in a directory of their own.
#[derive(Clone)]
pub struct Dir {
  store: Arc<dyn CloudStore>,
  /// No leading or trailing slash, empty for the root.
  path: String,
}

impl Dir {
  pub fn root(store: impl Into<Arc<dyn CloudStore>>) -> Self {
    Self {
      store: store.into(),
      path: String::new(),
    }
  }

  pub fn path(&self) -> &str {
    &self.path
  }

  /// A subdirectory, which may be several levels deep.
  pub fn dir(&self, path: &str) -> Result<Dir> {
    validate_key(path)?;
    Ok(Dir {
      store: self.store.clone(),
      path: self.join(path),
    })
  }

  pub(crate) fn key(&self, name: &str) -> Result<String> {
    validate_key(name)?;
    Ok(self.join(name))
  }

  fn join(&self, rel: &str) -> String {
    if self.path.is_empty() {
      rel.to_string()
    } else {
      format!("{}/{rel}", self.path)
    }
  }

  fn base(&self) -> String {
    if self.path.is_empty() {
      String::new()
    } else {
      format!("{}/", self.path)
    }
  }

  fn relative(&self, mut meta: ObjectMeta) -> ObjectMeta {
    meta.key = meta.key[self.base().len()..].to_string();
    meta
  }

  /// Everything below the directory whose relative name starts with `partial`.
  fn scan(&self, partial: &str) -> Result<Vec<ObjectMeta>> {
    let metas = self.store.list(&format!("{}{partial}", self.base()))?;
    Ok(metas.into_iter().map(|m| self.relative(m)).collect())
  }

  /// Files directly in the directory, sorted by name.
  pub fn list(&self) -> Result<Vec<ObjectMeta>> {
    self.list_prefix("")
  }

  /// Files directly in the directory whose name starts with `partial`, sorted by name.
  pub fn list_prefix(&self, partial: &str) -> Result<Vec<ObjectMeta>> {
    if partial.contains('/') {
      return Err(StoreError::InvalidKey(format!("{partial:?} (prefix spans directories)")).into());
    }
    Ok(
      self
        .scan(partial)?
        .into_iter()
        .filter(|m| !m.key.contains('/'))
        .collect(),
    )
  }

  /// Every file below the directory at any depth, sorted by name.
  pub fn list_all(&self) -> Result<Vec<ObjectMeta>> {
    self.scan("")
  }

  /// Names of the subdirectories that hold at least one file.
  pub fn list_dirs(&self) -> Result<Vec<String>> {
    let dirs: BTreeSet<String> = self
      .scan("")?
      .into_iter()
      .filter_map(|m| m.key.split_once('/').map(|(d, _)| d.to_string()))
      .collect();
    Ok(dirs.into_iter().collect())
  }

  pub fn stat(&self, name: &str) -> Result<Option<ObjectMeta>> {
    Ok(self.store.stat(&self.key(name)?)?.map(|m| self.relative(m)))
  }

  pub fn exists(&self, name: &str) -> Result<bool> {
    Ok(self.stat(name)?.is_some())
  }

  pub fn get_bytes(&self, name: &str) -> Result<Vec<u8>> {
    Ok(self.store.get(&self.key(name)?)?)
  }

  /// Creates the file or replaces it.
  pub fn put_bytes(&self, name: &str, data: &[u8]) -> Result<ObjectMeta> {
    Ok(self.relative(self.store.put(&self.key(name)?, data)?))
  }

  /// A missing file is not an error.
  pub fn delete(&self, name: &str) -> Result<()> {
    Ok(self.store.delete(&self.key(name)?)?)
  }

  /// Deletes every file below the directory and returns how many there were.
  /// The root is refused, because wiping the whole store is never a side effect to risk.
  pub fn delete_dir(&self) -> Result<usize> {
    if self.path.is_empty() {
      return Err(StoreError::InvalidKey("refusing to delete the root directory".into()).into());
    }
    let metas = self.scan("")?;
    for m in &metas {
      self.delete(&m.key)?;
    }
    log::debug!("deleted {} files below {}", metas.len(), self.path);
    Ok(metas.len())
  }

  pub(crate) fn already_exists(&self, name: &str) -> Error {
    Error::AlreadyExists(self.join(name))
  }
}
