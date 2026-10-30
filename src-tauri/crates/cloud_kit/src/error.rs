use std::{fmt, io};

#[derive(Debug)]
pub enum Error {
  Store(cloud_sync::Error),
  Json(serde_json::Error),
  Io(io::Error),
  /// A create found the name taken.
  AlreadyExists(String),
  /// A blob whose manifest, chunks and checksum do not agree.
  Corrupt(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
  pub fn is_not_found(&self) -> bool {
    matches!(self, Error::Store(cloud_sync::Error::NotFound(_)))
  }
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Error::Store(e) => write!(f, "{e}"),
      Error::Json(e) => write!(f, "json error: {e}"),
      Error::Io(e) => write!(f, "io error: {e}"),
      Error::AlreadyExists(key) => write!(f, "already exists: {key}"),
      Error::Corrupt(why) => write!(f, "corrupt blob: {why}"),
    }
  }
}

impl std::error::Error for Error {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    match self {
      Error::Store(e) => Some(e),
      Error::Json(e) => Some(e),
      Error::Io(e) => Some(e),
      _ => None,
    }
  }
}

impl From<cloud_sync::Error> for Error {
  fn from(e: cloud_sync::Error) -> Self {
    Error::Store(e)
  }
}

impl From<serde_json::Error> for Error {
  fn from(e: serde_json::Error) -> Self {
    Error::Json(e)
  }
}

impl From<io::Error> for Error {
  fn from(e: io::Error) -> Self {
    Error::Io(e)
  }
}
