use std::{fmt, io};

#[derive(Debug)]
pub enum Error {
  Db(sqlx::Error),
  Io(io::Error),
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
      Error::Migrate(e) => write!(f, "migration error: {e}"),
    }
  }
}

impl std::error::Error for Error {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    match self {
      Error::Db(e) => Some(e),
      Error::Io(e) => Some(e),
      Error::Migrate(e) => Some(e),
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

impl From<sqlx::migrate::MigrateError> for Error {
  fn from(e: sqlx::migrate::MigrateError) -> Self {
    Error::Migrate(e)
  }
}
