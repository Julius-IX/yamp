//! Serves audio files of this device to the webview over loopback HTTP.
//!
//! The webview could read a file through Tauri's asset protocol, but that is a custom URL scheme
//! and the media engine of WebKitGTK does not play from one. A plain `http://127.0.0.1` URL works
//! on every desktop webview. The server knows no paths. It is given an id and asks a resolver
//! which file that is, so only what the app wants to play can be reached.
//!
//! Every URL carries a token that is made at start. The port is open to other programs of the
//! machine, and a page in a browser could try it as well, so a request without the token gets
//! nothing. The token is only valid for one run.
//!
//! A media element asks for byte ranges to seek and to start before the file is complete, so
//! `Range` is answered as `206`. Each request gets a thread, since a reader that stops in the
//! middle of a file would otherwise hold up every request behind it.

use std::{
  collections::hash_map::RandomState,
  fs::File,
  hash::{BuildHasher, Hasher},
  io::{self, Read, Seek, SeekFrom},
  path::{Path, PathBuf},
  sync::Arc,
  thread,
};

use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

type Body = Response<Box<dyn Read + Send>>;
type Resolver = dyn Fn(&str) -> Option<PathBuf> + Send + Sync;

/// A running server. It stops with the process.
pub struct FileServer {
  port: u16,
  token: String,
}

impl FileServer {
  /// Starts on a free port. `resolve` turns an id from a URL into a file, or `None` for an id
  /// that is not known.
  pub fn start(resolve: impl Fn(&str) -> Option<PathBuf> + Send + Sync + 'static) -> io::Result<Self> {
    let server = Server::http("127.0.0.1:0").map_err(io::Error::other)?;
    let port = server
      .server_addr()
      .to_ip()
      .map(|a| a.port())
      .ok_or_else(|| io::Error::other("the server has no port"))?;
    let token = make_token();
    let (resolve, own_token): (Arc<Resolver>, String) = (Arc::new(resolve), token.clone());

    thread::Builder::new().name("file-server".into()).spawn(move || {
      for request in server.incoming_requests() {
        let (resolve, token) = (resolve.clone(), own_token.clone());
        thread::spawn(move || {
          let response = answer(&request, &token, resolve.as_ref());
          let _ = request.respond(response);
        });
      }
    })?;
    log::info!("file server listening on 127.0.0.1:{port}");
    Ok(Self { port, token })
  }

  /// The URL that plays the file of `id`. Ids are limited to letters, digits, `-` and `_`.
  pub fn url(&self, id: &str) -> String {
    format!("http://127.0.0.1:{}/{}/{id}", self.port, self.token)
  }
}

/// Random per process, which is all that a token for a loopback port needs. The standard
/// library seeds its hasher from the operating system.
fn make_token() -> String {
  let (a, b) = (RandomState::new().build_hasher().finish(), RandomState::new().build_hasher().finish());
  format!("{a:016x}{b:016x}")
}

fn answer(request: &Request, token: &str, resolve: &Resolver) -> Body {
  if !matches!(request.method(), Method::Get | Method::Head) {
    return status(405);
  }
  let url = request.url();
  let path = url.split(['?', '#']).next().unwrap_or("");
  let mut parts = path.trim_start_matches('/').split('/');
  let (Some(given), Some(id), None) = (parts.next(), parts.next(), parts.next()) else {
    return status(404);
  };
  if given != token || !valid_id(id) {
    return status(404);
  }
  let Some(file) = resolve(id) else {
    return status(404);
  };
  let range = request
    .headers()
    .iter()
    .find(|h| h.field.equiv("Range"))
    .map(|h| h.value.as_str().to_owned());
  serve(&file, range.as_deref())
}

fn valid_id(id: &str) -> bool {
  !id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn serve(path: &Path, range: Option<&str>) -> Body {
  let Ok(mut file) = File::open(path) else {
    return status(404);
  };
  let Ok(len) = file.metadata().map(|m| m.len()) else {
    return status(500);
  };
  let (code, start, end) = match range.map(|r| parse_range(r, len)) {
    None | Some(Parsed::Ignore) => (200, 0, len.saturating_sub(1)),
    Some(Parsed::Range(start, end)) => (206, start, end),
    Some(Parsed::Unsatisfiable) => {
      let mut response = status(416);
      response.add_header(header("Content-Range", &format!("bytes */{len}")));
      return response;
    }
  };
  let count = if len == 0 { 0 } else { end - start + 1 };
  if start > 0 && file.seek(SeekFrom::Start(start)).is_err() {
    return status(500);
  }
  let body: Box<dyn Read + Send> = Box::new(file.take(count));
  let mut response = Response::new(StatusCode(code), Vec::new(), body, Some(count as usize), None);
  response.add_header(header("Content-Type", content_type(path)));
  response.add_header(header("Accept-Ranges", "bytes"));
  response.add_header(header("Access-Control-Allow-Origin", "*"));
  response.add_header(header("Cache-Control", "no-store"));
  if code == 206 {
    response.add_header(header("Content-Range", &format!("bytes {start}-{end}/{len}")));
  }
  response
}

#[derive(Debug, PartialEq, Eq)]
enum Parsed {
  /// Inclusive start and end.
  Range(u64, u64),
  /// A header that is not understood, or asks for several ranges, is answered with the whole file.
  Ignore,
  Unsatisfiable,
}

/// Handles `bytes=a-b`, `bytes=a-` and `bytes=-n`.
fn parse_range(value: &str, len: u64) -> Parsed {
  let Some(spec) = value.trim().strip_prefix("bytes=") else {
    return Parsed::Ignore;
  };
  if spec.contains(',') {
    return Parsed::Ignore;
  }
  let Some((from, to)) = spec.split_once('-') else {
    return Parsed::Ignore;
  };
  let (from, to) = (from.trim(), to.trim());
  if len == 0 {
    return Parsed::Unsatisfiable;
  }
  if from.is_empty() {
    // the last n bytes
    return match to.parse::<u64>() {
      Ok(n) if n > 0 => Parsed::Range(len.saturating_sub(n), len - 1),
      Ok(_) => Parsed::Unsatisfiable,
      Err(_) => Parsed::Ignore,
    };
  }
  let Ok(start) = from.parse::<u64>() else {
    return Parsed::Ignore;
  };
  let end = if to.is_empty() {
    len - 1
  } else {
    match to.parse::<u64>() {
      Ok(end) => end.min(len - 1),
      Err(_) => return Parsed::Ignore,
    }
  };
  if start >= len || start > end { Parsed::Unsatisfiable } else { Parsed::Range(start, end) }
}

fn content_type(path: &Path) -> &'static str {
  match path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref() {
    Some("mp3") => "audio/mpeg",
    Some("m4a") => "audio/mp4",
    Some("aac") => "audio/aac",
    Some("flac") => "audio/flac",
    Some("ogg" | "oga") => "audio/ogg",
    Some("opus") => "audio/ogg; codecs=opus",
    Some("wav") => "audio/wav",
    Some("weba" | "webm") => "audio/webm",
    _ => "application/octet-stream",
  }
}

fn header(name: &str, value: &str) -> Header {
  Header::from_bytes(name.as_bytes(), value.as_bytes()).expect("header names and values here are plain ASCII")
}

fn status(code: u16) -> Body {
  let empty: Box<dyn Read + Send> = Box::new(io::empty());
  Response::new(StatusCode(code), Vec::new(), empty, Some(0), None)
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::{io::Write, net::TcpStream};

  fn fixture(name: &str) -> (FileServer, PathBuf) {
    let dir = std::env::temp_dir().join(format!("file-server-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("song.mp3");
    std::fs::write(&file, (0..100u8).collect::<Vec<_>>()).unwrap();
    let served = file.clone();
    let server = FileServer::start(move |id| (id == "abc").then(|| served.clone())).unwrap();
    (server, dir)
  }

  /// A raw request, since the crate has no client. Returns the status line and the body.
  fn get(url: &str, range: Option<&str>) -> (String, Vec<u8>) {
    let rest = url.strip_prefix("http://").unwrap();
    let (host, path) = rest.split_once('/').unwrap();
    let mut stream = TcpStream::connect(host).unwrap();
    let range = range.map(|r| format!("Range: {r}\r\n")).unwrap_or_default();
    write!(stream, "GET /{path} HTTP/1.1\r\nHost: {host}\r\n{range}Connection: close\r\n\r\n").unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).unwrap();
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    let head = String::from_utf8_lossy(&raw[..split]).into_owned();
    (head.lines().next().unwrap().to_owned(), raw[split + 4..].to_vec())
  }

  #[test]
  fn serves_the_whole_file_and_ranges_of_it() {
    let (server, dir) = fixture("ranges");
    let url = server.url("abc");

    let (line, body) = get(&url, None);
    assert!(line.contains("200"), "{line}");
    assert_eq!(body, (0..100u8).collect::<Vec<_>>());

    let (line, body) = get(&url, Some("bytes=10-19"));
    assert!(line.contains("206"), "{line}");
    assert_eq!(body, (10..20u8).collect::<Vec<_>>());

    let (_, body) = get(&url, Some("bytes=95-"));
    assert_eq!(body, (95..100u8).collect::<Vec<_>>());
    let (_, body) = get(&url, Some("bytes=-3"));
    assert_eq!(body, (97..100u8).collect::<Vec<_>>());

    let (line, _) = get(&url, Some("bytes=500-"));
    assert!(line.contains("416"), "{line}");
    let _ = std::fs::remove_dir_all(dir);
  }

  #[test]
  fn refuses_a_wrong_token_and_an_unknown_id() {
    let (server, dir) = fixture("refuse");
    let url = server.url("abc");

    let (line, _) = get(&url.replace(&server.token, "wrong"), None);
    assert!(line.contains("404"), "{line}");
    let (line, _) = get(&server.url("nope"), None);
    assert!(line.contains("404"), "{line}");
    let (line, _) = get(&server.url("../etc"), None);
    assert!(line.contains("404"), "{line}");
    let _ = std::fs::remove_dir_all(dir);
  }

  #[test]
  fn can_be_shared_between_threads() {
    fn shared<T: Send + Sync>() {}
    shared::<FileServer>();
  }

  #[test]
  fn ranges_are_parsed() {
    assert_eq!(parse_range("bytes=0-9", 100), Parsed::Range(0, 9));
    assert_eq!(parse_range("bytes=90-500", 100), Parsed::Range(90, 99));
    assert_eq!(parse_range("bytes=0-1,5-6", 100), Parsed::Ignore);
    assert_eq!(parse_range("items=0-1", 100), Parsed::Ignore);
    assert_eq!(parse_range("bytes=100-", 100), Parsed::Unsatisfiable);
    assert_eq!(parse_range("bytes=5-2", 100), Parsed::Unsatisfiable);
  }
}
