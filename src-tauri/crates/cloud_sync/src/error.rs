use std::{fmt, io, time::Duration};

/// Failure of a storage operation.
#[derive(Debug)]
pub enum Error {
  /// The key breaks the portable naming rules, see [`validate_key`](crate::validate_key).
  InvalidKey(String),
  NotFound(String),
  /// The object is known but its content is not on the device yet. Only iCloud
  /// produces this, the download has to be started through the native API.
  NotDownloaded(String),
  TooLarge {
    size: usize,
    max: usize,
  },
  /// The token was rejected again after a refresh.
  Unauthorized,
  RateLimited {
    retry_after: Option<Duration>,
  },
  /// A backend needs a setting that was not provided.
  MissingSetting(&'static str),
  Http {
    status: u16,
    message: String,
  },
  Transport(String),
  /// A response that does not have the documented shape.
  Protocol(String),
  Io(io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Error::InvalidKey(why) => write!(f, "invalid key: {why}"),
      Error::NotFound(key) => write!(f, "object not found: {key}"),
      Error::NotDownloaded(key) => write!(f, "object not downloaded yet: {key}"),
      Error::TooLarge { size, max } => {
        write!(f, "object of {size} bytes exceeds the {max} byte limit")
      }
      Error::Unauthorized => write!(f, "access token was rejected"),
      Error::RateLimited {
        retry_after: Some(d),
      } => write!(f, "rate limited, retry after {}s", d.as_secs()),
      Error::RateLimited { retry_after: None } => write!(f, "rate limited"),
      Error::MissingSetting(name) => write!(f, "missing setting: {name}"),
      Error::Http { status, message } => write!(f, "http {status}: {message}"),
      Error::Transport(msg) => write!(f, "transport error: {msg}"),
      Error::Protocol(msg) => write!(f, "unexpected response: {msg}"),
      Error::Io(e) => write!(f, "io error: {e}"),
    }
  }
}

impl std::error::Error for Error {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    match self {
      Error::Io(e) => Some(e),
      _ => None,
    }
  }
}

impl From<io::Error> for Error {
  fn from(e: io::Error) -> Self {
    Error::Io(e)
  }
}
