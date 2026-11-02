use std::{fmt, io};

#[derive(Debug)]
pub enum Error {
  Db(sqlx::Error),
  Io(io::Error),
  Json(serde_json::Error),
  /// An edit named something that does not exist, such as a playlist id.
  NotFound(String),
  /// An edit that makes no sense, such as a position past the end of a list.
  Invalid(String),
  /// The schema in the database does not fit the migrations of this build, for example
  /// because a newer version of the app wrote it.
  Migrate(sqlx::migrate::MigrateError),
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Error::Db(e) => write!(f, "database error: {e}"),
      Error::Io(e) => write!(f, "io error: {e}"),
      Error::Json(e) => write!(f, "json error: {e}"),
      Error::NotFound(what) => write!(f, "not found: {what}"),
      Error::Invalid(why) => write!(f, "invalid: {why}"),
      Error::Migrate(e) => write!(f, "migration error: {e}"),
    }
  }
}

impl std::error::Error for Error {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    match self {
      Error::Db(e) => Some(e),
      Error::Io(e) => Some(e),
      Error::Json(e) => Some(e),
      Error::Migrate(e) => Some(e),
      Error::NotFound(_) | Error::Invalid(_) => None,
    }
  }
}

impl From<sqlx::Error> for Error {
  fn from(e: sqlx::Error) -> Self {
    Error::Db(e)
  }
}

impl From<io::Error> for Error {
  fn from(e: io::Error) -> Self {
    Error::Io(e)
  }
}

impl From<serde_json::Error> for Error {
  fn from(e: serde_json::Error) -> Self {
    Error::Json(e)
  }
}

impl From<sqlx::migrate::MigrateError> for Error {
  fn from(e: sqlx::migrate::MigrateError) -> Self {
    Error::Migrate(e)
  }
}
