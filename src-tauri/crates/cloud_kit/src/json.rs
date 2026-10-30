//! JSON documents, as whole files in a [`Dir`].

use cloud_sync::ObjectMeta;
use serde::{de::DeserializeOwned, Serialize};

use crate::{Dir, Result};

impl Dir {
  /// Creates the document or replaces it.
  pub fn put_json<T: Serialize>(&self, name: &str, value: &T) -> Result<ObjectMeta> {
    self.put_bytes(name, &serde_json::to_vec(value)?)
  }

  pub fn get_json<T: DeserializeOwned>(&self, name: &str) -> Result<T> {
    Ok(serde_json::from_slice(&self.get_bytes(name)?)?)
  }

  /// `None` when the document does not exist.
  pub fn get_json_opt<T: DeserializeOwned>(&self, name: &str) -> Result<Option<T>> {
    match self.get_json(name) {
      Ok(value) => Ok(Some(value)),
      Err(e) if e.is_not_found() => Ok(None),
      Err(e) => Err(e),
    }
  }

  /// Creates the document and fails with `AlreadyExists` if the name is taken.
  /// The check and the write are two requests, since no backend has an atomic create.
  /// Writers are expected to own their names, as everywhere in this crate.
  pub fn insert_json<T: Serialize>(&self, name: &str, value: &T) -> Result<ObjectMeta> {
    if self.exists(name)? {
      return Err(self.already_exists(name));
    }
    self.put_json(name, value)
  }

  /// Reads the document, applies `change` and writes the result back.
  /// Fails with `NotFound` if the document does not exist.
  pub fn update_json<T, F>(&self, name: &str, change: F) -> Result<T>
  where
    T: Serialize + DeserializeOwned,
    F: FnOnce(T) -> T,
  {
    let updated = change(self.get_json(name)?);
    self.put_json(name, &updated)?;
    Ok(updated)
  }
}
