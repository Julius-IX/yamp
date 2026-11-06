use std::{fmt, io};

#[derive(Debug)]
pub enum Error {
  /// The build has no OAuth client for the service. `vars` lists the variables that were looked at.
  NotConfigured { service: &'static str, vars: String },
  /// The service has no sign in, iCloud works through the operating system instead.
  NoSignIn(&'static str),
  /// The user turned the request down or the service refused it.
  Denied(String),
  Cancelled,
  TimedOut,
  /// The stored refresh token is no longer valid, the user has to sign in again.
  Revoked,
  NotSignedIn,
  Transport(String),
  Protocol(String),
  Io(io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Error::NotConfigured { service, vars } => {
        write!(f, "{service} is not set up in this build, none of {vars} has a value")
      }
      Error::NoSignIn(service) => write!(f, "{service} has no sign in"),
      Error::Denied(why) => write!(f, "the sign in was refused: {why}"),
      Error::Cancelled => write!(f, "the sign in was cancelled"),
      Error::TimedOut => write!(f, "the sign in took too long"),
      Error::Revoked => write!(f, "the access was revoked, sign in again"),
      Error::NotSignedIn => write!(f, "not signed in"),
      Error::Transport(why) => write!(f, "could not reach the service: {why}"),
      Error::Protocol(why) => write!(f, "unexpected answer from the service: {why}"),
      Error::Io(e) => write!(f, "{e}"),
    }
  }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
  fn from(e: io::Error) -> Self {
    Error::Io(e)
  }
}
