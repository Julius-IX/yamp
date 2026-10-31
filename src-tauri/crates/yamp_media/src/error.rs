use std::fmt;

pub type Result<T> = std::result::Result<T, MediaError>;

#[derive(Debug)]
pub enum MediaError {
  /// The provider tooling is missing. Callers can install it with `Updater::update`.
  NotInstalled,
  /// The request was understood but the provider returned nothing usable.
  NotFound,
  /// An identifier that could never be valid. Rejected before reaching the provider.
  InvalidId,
  /// The provider tooling ran and failed. Carries its own message.
  Engine(String),
  Io(String),
  Network(String),
  Parse(String),
}

impl fmt::Display for MediaError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::NotInstalled => f.write_str("the media engine is not installed yet"),
      Self::NotFound => f.write_str("nothing found"),
      Self::InvalidId => f.write_str("invalid id"),
      Self::Engine(m) | Self::Io(m) | Self::Network(m) | Self::Parse(m) => f.write_str(m),
    }
  }
}

impl std::error::Error for MediaError {}

impl From<std::io::Error> for MediaError {
  fn from(e: std::io::Error) -> Self {
    Self::Io(e.to_string())
  }
}

impl From<serde_json::Error> for MediaError {
  fn from(e: serde_json::Error) -> Self {
    Self::Parse(e.to_string())
  }
}
