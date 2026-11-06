use std::{
  io::{ErrorKind, Read, Write},
  net::{TcpListener, TcpStream},
  sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{Receiver, RecvTimeoutError},
  },
  thread,
  time::{Duration, Instant},
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use cloud_sync::Provider;
use percent_encoding::{NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};
use sha2::{Digest, Sha256};

use crate::{
  Error, Result, SignedIn,
  app::{OAuthApp, Redirect, app},
  link,
  token::exchange,
};

const POLL: Duration = Duration::from_millis(100);
const READ_TIMEOUT: Duration = Duration::from_secs(2);

const DONE_PAGE: &str = "<!doctype html><meta charset=utf-8><title>yamp</title>\
<body style=\"font-family:sans-serif;text-align:center;margin-top:20vh\">\
<h2>Signed in</h2><p>You can close this tab and go back to yamp.</p>";
const FAILED_PAGE: &str = "<!doctype html><meta charset=utf-8><title>yamp</title>\
<body style=\"font-family:sans-serif;text-align:center;margin-top:20vh\">\
<h2>Sign in failed</h2><p>Go back to yamp and try again.</p>";

/// A sign in that waits for the browser to come back.
pub struct Pending {
  /// The page of the service the user has to visit.
  pub url: String,
  transport: Transport,
  app: OAuthApp,
  verifier: String,
  state: String,
  redirect: String,
}

/// Where the answer arrives, see [`Redirect`].
enum Transport {
  Loopback(TcpListener),
  Link(Receiver<String>),
}

fn random_token(bytes: usize) -> Result<String> {
  let mut buf = vec![0u8; bytes];
  getrandom::getrandom(&mut buf).map_err(|e| Error::Protocol(e.to_string()))?;
  Ok(URL_SAFE_NO_PAD.encode(buf))
}

fn challenge(verifier: &str) -> String {
  URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

fn encode_form(pairs: &[(&str, &str)]) -> String {
  pairs
    .iter()
    .map(|(k, v)| format!("{k}={}", utf8_percent_encode(v, NON_ALPHANUMERIC)))
    .collect::<Vec<_>>()
    .join("&")
}

/// Starts a sign in. Nothing leaves the machine until the user opens the returned url. `app_id` is
/// the identifier of the app, a mobile sign in uses it as the scheme of the link it is sent back by.
pub fn begin(provider: Provider, app_id: &str) -> Result<Pending> {
  let app = app(provider, app_id)?;
  let verifier = random_token(32)?;
  let state = random_token(16)?;

  let (redirect, transport) = match &app.redirect {
    Redirect::Loopback { host } => {
      // port 0 lets the system pick a free one, which the service accepts for a loopback redirect
      let listener = TcpListener::bind(("127.0.0.1", 0))?;
      (format!("http://{host}:{}", listener.local_addr()?.port()), Transport::Loopback(listener))
    }
    Redirect::Scheme(uri) => (uri.clone(), Transport::Link(link::register(uri, &state))),
  };

  let code_challenge = challenge(&verifier);
  let mut query = vec![
    ("client_id", app.client_id),
    ("redirect_uri", redirect.as_str()),
    ("response_type", "code"),
    ("scope", app.scope),
    ("code_challenge", code_challenge.as_str()),
    ("code_challenge_method", "S256"),
    ("state", state.as_str()),
  ];
  query.extend_from_slice(app.extra);
  let url = format!("{}?{}", app.auth_url, encode_form(&query));
  Ok(Pending { url, transport, app, verifier, state, redirect })
}

impl Pending {
  /// Blocks until the browser is back, the sign in is cancelled or `timeout` passes. Run it on a
  /// thread that may block.
  pub fn wait(self, timeout: Duration, cancel: &AtomicBool) -> Result<SignedIn> {
    let code = match &self.transport {
      Transport::Loopback(listener) => accept_code(listener, &self.state, timeout, cancel),
      Transport::Link(rx) => {
        let code = wait_for_link(rx, timeout, cancel);
        link::clear();
        code
      }
    }?;
    exchange(&self.app, &code, &self.verifier, &self.redirect)
  }
}

/// The counterpart of `accept_code` for a sign in that is answered with a link, see
/// [`deliver_redirect`](crate::deliver_redirect). A link only gets here when its state matched.
fn wait_for_link(rx: &Receiver<String>, timeout: Duration, cancel: &AtomicBool) -> Result<String> {
  let deadline = Instant::now() + timeout;
  loop {
    if cancel.load(Ordering::Relaxed) {
      return Err(Error::Cancelled);
    }
    if Instant::now() >= deadline {
      return Err(Error::TimedOut);
    }
    match rx.recv_timeout(POLL) {
      Ok(url) => {
        let params = parse_query(url.split_once('?').map_or("", |(_, q)| q));
        let get = |k: &str| params.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
        if let Some(error) = get("error") {
          return Err(Error::Denied(get("error_description").unwrap_or(error).to_owned()));
        }
        if let Some(code) = get("code") {
          return Ok(code.to_owned());
        }
      }
      Err(RecvTimeoutError::Timeout) => {}
      // a newer sign in took the place of this one
      Err(RecvTimeoutError::Disconnected) => return Err(Error::Cancelled),
    }
  }
}

fn accept_code(listener: &TcpListener, state: &str, timeout: Duration, cancel: &AtomicBool) -> Result<String> {
  // polling is what lets a cancel be noticed, a blocking accept could not be interrupted
  listener.set_nonblocking(true)?;
  let deadline = Instant::now() + timeout;
  loop {
    if cancel.load(Ordering::Relaxed) {
      return Err(Error::Cancelled);
    }
    if Instant::now() >= deadline {
      return Err(Error::TimedOut);
    }
    match listener.accept() {
      Ok((stream, _)) => {
        if let Some(answer) = handle(stream, state)? {
          return answer;
        }
      }
      Err(e) if e.kind() == ErrorKind::WouldBlock => thread::sleep(POLL),
      Err(e) => return Err(e.into()),
    }
  }
}

/// `None` means the request was not the redirect, a browser also asks for a favicon for one.
fn handle(mut stream: TcpStream, state: &str) -> Result<Option<Result<String>>> {
  stream.set_nonblocking(false)?;
  stream.set_read_timeout(Some(READ_TIMEOUT))?;
  let mut buf = [0u8; 8192];
  let n = stream.read(&mut buf).unwrap_or(0);
  let request = String::from_utf8_lossy(&buf[..n]);
  let target = request.lines().next().and_then(|l| l.split(' ').nth(1)).unwrap_or("");
  let (path, query) = target.split_once('?').unwrap_or((target, ""));
  let params = parse_query(query);
  let get = |k: &str| params.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());

  // anything that does not carry the state of this sign in is not an answer to it
  if path != "/" || get("state") != Some(state) {
    respond(&mut stream, "404 Not Found", "");
    return Ok(None);
  }
  if let Some(error) = get("error") {
    respond(&mut stream, "200 OK", FAILED_PAGE);
    let why = get("error_description").unwrap_or(error);
    return Ok(Some(Err(Error::Denied(why.to_owned()))));
  }
  match get("code") {
    Some(code) => {
      respond(&mut stream, "200 OK", DONE_PAGE);
      Ok(Some(Ok(code.to_owned())))
    }
    None => {
      respond(&mut stream, "400 Bad Request", FAILED_PAGE);
      Ok(None)
    }
  }
}

fn respond(stream: &mut TcpStream, status: &str, body: &str) {
  let head = format!(
    "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
    body.len()
  );
  // the browser may already be gone, and the sign in does not depend on this page arriving
  let _ = stream.write_all(head.as_bytes()).and_then(|_| stream.write_all(body.as_bytes()));
}

pub(crate) fn parse_query(query: &str) -> Vec<(String, String)> {
  query
    .split('&')
    .filter_map(|pair| pair.split_once('='))
    .map(|(k, v)| {
      let decode = |s: &str| percent_decode_str(&s.replace('+', " ")).decode_utf8_lossy().into_owned();
      (decode(k), decode(v))
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  fn visit(port: u16, target: &str) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(s, "GET {target} HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    let mut out = String::new();
    let _ = s.read_to_string(&mut out);
  }

  #[test]
  fn challenge_matches_the_rfc_example() {
    // RFC 7636, appendix B
    assert_eq!(
      challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
      "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    );
  }

  #[test]
  fn the_code_is_taken_from_the_redirect() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    let t = thread::spawn(move || {
      visit(port, "/favicon.ico");
      visit(port, "/?code=nope&state=other");
      visit(port, "/?code=a%2Fb+c&state=s1");
    });
    let code = accept_code(&listener, "s1", Duration::from_secs(5), &AtomicBool::new(false)).unwrap();
    t.join().unwrap();
    assert_eq!(code, "a/b c");
  }

  // one sign in can wait for a link at a time, so these tests may not overlap
  static LINK_TESTS: std::sync::Mutex<()> = std::sync::Mutex::new(());

  #[test]
  fn a_link_with_the_right_state_is_the_answer() {
    let _one_at_a_time = LINK_TESTS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let rx = link::register("com.x.y://auth", "s1");
    assert!(!crate::deliver_redirect("com.other://auth?code=x&state=s1"));
    assert!(!crate::deliver_redirect("com.x.y://auth?code=x&state=other"));
    assert!(crate::deliver_redirect("com.x.y://auth?code=a%2Fb&state=s1"));
    let code = wait_for_link(&rx, Duration::from_secs(5), &AtomicBool::new(false));
    link::clear();
    assert_eq!(code.unwrap(), "a/b");
    assert!(!crate::deliver_redirect("com.x.y://auth?code=late&state=s1"));
  }

  #[test]
  fn a_refused_link_a_cancel_and_a_replaced_sign_in_end_the_wait() {
    let _one_at_a_time = LINK_TESTS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let rx = link::register("com.x.y://auth", "s1");
    assert!(crate::deliver_redirect("com.x.y://auth?error=access_denied&state=s1"));
    let denied = wait_for_link(&rx, Duration::from_secs(5), &AtomicBool::new(false));
    assert!(matches!(denied, Err(Error::Denied(why)) if why == "access_denied"));

    let rx = link::register("com.x.y://auth", "s2");
    assert!(matches!(wait_for_link(&rx, Duration::from_secs(5), &AtomicBool::new(true)), Err(Error::Cancelled)));

    let old = link::register("com.x.y://auth", "s3");
    let _new = link::register("com.x.y://auth", "s4");
    assert!(matches!(wait_for_link(&old, Duration::from_secs(5), &AtomicBool::new(false)), Err(Error::Cancelled)));
    link::clear();
  }

  #[test]
  fn a_refusal_and_a_cancel_end_the_wait() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    let t = thread::spawn(move || visit(port, "/?error=access_denied&state=s1"));
    let denied = accept_code(&listener, "s1", Duration::from_secs(5), &AtomicBool::new(false));
    t.join().unwrap();
    assert!(matches!(denied, Err(Error::Denied(why)) if why == "access_denied"));

    let cancelled = accept_code(&listener, "s1", Duration::from_secs(5), &AtomicBool::new(true));
    assert!(matches!(cancelled, Err(Error::Cancelled)));
  }
}
