//! Request plumbing shared by the REST backends. It adds the bearer token, refreshes
//! it once on a 401 and waits out throttling.

use std::{io::Read, sync::Arc, thread, time::Duration};

use serde::de::DeserializeOwned;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::{store::MAX_OBJECT_SIZE, Error, Result, TokenProvider};

const MAX_RETRIES: u32 = 3;
const MAX_WAIT: Duration = Duration::from_secs(30);
const MAX_ERROR_BODY: usize = 500;

pub(crate) enum Body<'a> {
  Empty,
  Bytes {
    content_type: &'a str,
    data: &'a [u8],
  },
}

pub(crate) struct Http {
  agent: ureq::Agent,
  // follows no redirects, see `call_no_redirect`
  bare: ureq::Agent,
  tokens: Arc<dyn TokenProvider>,
}

impl Http {
  pub fn new(tokens: Arc<dyn TokenProvider>) -> Self {
    let builder = || {
      ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout(Duration::from_secs(60))
    };
    Self {
      agent: builder().build(),
      bare: builder().redirects(0).build(),
      tokens,
    }
  }

  /// `build` runs once per attempt, because a sent request cannot be reused.
  pub fn call(
    &self,
    build: impl Fn(&ureq::Agent) -> ureq::Request,
    body: Body<'_>,
  ) -> Result<ureq::Response> {
    self.execute(&self.agent, build, body)
  }

  /// Hands back a 3xx response as is. Download links that a service redirects to are
  /// pre-authorized and some reject an extra `Authorization` header.
  pub fn call_no_redirect(
    &self,
    build: impl Fn(&ureq::Agent) -> ureq::Request,
    body: Body<'_>,
  ) -> Result<ureq::Response> {
    self.execute(&self.bare, build, body)
  }

  /// A request without credentials, for pre-authorized links.
  pub fn plain_get(&self, url: &str) -> Result<ureq::Response> {
    self.agent.get(url).call().map_err(convert)
  }

  fn execute(
    &self,
    agent: &ureq::Agent,
    build: impl Fn(&ureq::Agent) -> ureq::Request,
    body: Body<'_>,
  ) -> Result<ureq::Response> {
    let mut token = self.tokens.token(false)?;
    let mut refreshed = false;
    let mut attempt = 0;
    loop {
      let req = build(agent).set("Authorization", &format!("Bearer {token}"));
      let sent = match &body {
        Body::Empty => req.call(),
        Body::Bytes { content_type, data } => {
          req.set("Content-Type", content_type).send_bytes(data)
        }
      };
      let err = match sent {
        Ok(res) => return Ok(res),
        Err(e) => e,
      };
      // transport errors are not retried, a create that did reach the server would be repeated
      match &err {
        ureq::Error::Status(401, _) if !refreshed => {
          log::debug!("token was rejected, refreshing it");
          token = self.tokens.token(true)?;
          refreshed = true;
        }
        ureq::Error::Status(code, res)
          if attempt < MAX_RETRIES && matches!(*code, 429 | 502 | 503 | 504) =>
        {
          let wait = retry_after(res)
            .unwrap_or(Duration::from_millis(500 << attempt))
            .min(MAX_WAIT);
          log::warn!(
            "http {code}, retry {} of {MAX_RETRIES} in {}ms",
            attempt + 1,
            wait.as_millis()
          );
          thread::sleep(wait);
          attempt += 1;
        }
        _ => return Err(convert(err)),
      }
    }
  }
}

fn convert(e: ureq::Error) -> Error {
  match e {
    ureq::Error::Status(401, _) => Error::Unauthorized,
    ureq::Error::Status(429, res) => Error::RateLimited {
      retry_after: retry_after(&res),
    },
    ureq::Error::Status(status, res) => {
      let mut message = res.into_string().unwrap_or_default();
      message.truncate(floor_char_boundary(&message, MAX_ERROR_BODY));
      Error::Http { status, message }
    }
    ureq::Error::Transport(t) => Error::Transport(t.to_string()),
  }
}

fn retry_after(res: &ureq::Response) -> Option<Duration> {
  res
    .header("Retry-After")?
    .trim()
    .parse()
    .ok()
    .map(Duration::from_secs)
}

pub(crate) fn json<T: DeserializeOwned>(res: ureq::Response) -> Result<T> {
  res.into_json().map_err(|e| Error::Protocol(e.to_string()))
}

/// Reads a body up to the object size limit, so a huge file cannot fill the memory.
pub(crate) fn read_body(res: ureq::Response) -> Result<Vec<u8>> {
  let mut buf = Vec::new();
  res
    .into_reader()
    .take(MAX_OBJECT_SIZE as u64 + 1)
    .read_to_end(&mut buf)?;
  if buf.len() > MAX_OBJECT_SIZE {
    return Err(Error::TooLarge {
      size: buf.len(),
      max: MAX_OBJECT_SIZE,
    });
  }
  Ok(buf)
}

/// Turns the 404 of a lookup by key into [`Error::NotFound`].
pub(crate) fn not_found(e: Error, key: &str) -> Error {
  match e {
    Error::Http { status: 404, .. } => Error::NotFound(key.to_string()),
    other => other,
  }
}

pub(crate) fn rfc3339_ms(s: &str) -> Option<i64> {
  let t = OffsetDateTime::parse(s, &Rfc3339).ok()?;
  i64::try_from(t.unix_timestamp_nanos() / 1_000_000).ok()
}

// `str::floor_char_boundary` is not available on older toolchains
fn floor_char_boundary(s: &str, max: usize) -> usize {
  let mut i = max.min(s.len());
  while !s.is_char_boundary(i) {
    i -= 1;
  }
  i
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_service_timestamps() {
    assert_eq!(rfc3339_ms("1970-01-01T00:00:01.250Z"), Some(1250));
    assert_eq!(
      rfc3339_ms("2024-03-05T10:20:30+01:00"),
      Some(1_709_630_430_000)
    );
    assert_eq!(rfc3339_ms("not a date"), None);
  }

  #[test]
  fn truncates_on_char_boundary() {
    assert_eq!(floor_char_boundary("aé", 2), 1);
    assert_eq!(floor_char_boundary("aé", 10), 3);
  }
}
