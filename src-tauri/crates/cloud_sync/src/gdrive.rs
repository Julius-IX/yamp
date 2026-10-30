//! Google Drive backend on the hidden `appDataFolder` space.
//!
//! That space is private to the app and invisible in the user's Drive. Drive allows
//! several files with one name, so a key is stored as a flat file name (slashes
//! included) and lookups take the newest match. Deleting removes all matches.

use std::sync::Arc;

use serde::Deserialize;

use crate::{
  http::{self, Body, Http},
  key::{validate_key, validate_prefix},
  store::check_size,
  CloudStore, Error, ObjectMeta, Result, Settings, TokenProvider,
};

const API: &str = "https://www.googleapis.com/drive/v3";
const UPLOAD: &str = "https://www.googleapis.com/upload/drive/v3";
const FILE_FIELDS: &str = "id,name,size,modifiedTime,md5Checksum,version";
const OCTET: &str = "application/octet-stream";

pub(crate) struct GoogleDriveStore {
  http: Http,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DriveFile {
  id: String,
  name: String,
  // Drive sends 64 bit sizes as strings
  size: Option<String>,
  modified_time: Option<String>,
  md5_checksum: Option<String>,
  version: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileList {
  files: Vec<DriveFile>,
  next_page_token: Option<String>,
}

impl DriveFile {
  fn into_meta(self) -> ObjectMeta {
    ObjectMeta {
      key: self.name,
      size: self.size.and_then(|s| s.parse().ok()).unwrap_or(0),
      modified_ms: self.modified_time.as_deref().and_then(http::rfc3339_ms),
      // the checksum only moves with the content, the version also moves with metadata
      revision: self.md5_checksum.or(self.version).unwrap_or_default(),
    }
  }
}

impl GoogleDriveStore {
  /// Files matching a Drive query, newest first.
  fn query(&self, q: &str) -> Result<Vec<DriveFile>> {
    let fields = format!("nextPageToken,files({FILE_FIELDS})");
    let mut files = Vec::new();
    let mut page: Option<String> = None;
    loop {
      let res = self.http.call(
        |a| {
          let req = a
            .get(&format!("{API}/files"))
            .query("spaces", "appDataFolder")
            .query("q", q)
            .query("orderBy", "modifiedTime desc")
            .query("pageSize", "1000")
            .query("fields", &fields);
          match &page {
            Some(token) => req.query("pageToken", token),
            None => req,
          }
        },
        Body::Empty,
      )?;
      let list: FileList = http::json(res)?;
      files.extend(list.files);
      match list.next_page_token {
        Some(token) => page = Some(token),
        None => return Ok(files),
      }
    }
  }

  fn find(&self, key: &str) -> Result<Vec<DriveFile>> {
    let files = self.query(&format!("name = '{}' and trashed = false", escape(key)))?;
    if files.len() > 1 {
      log::warn!("{key} exists {} times, the newest is used", files.len());
    }
    Ok(files)
  }
}

impl CloudStore for GoogleDriveStore {
  fn list(&self, prefix: &str) -> Result<Vec<ObjectMeta>> {
    validate_prefix(prefix)?;
    // `name contains` matches words and not prefixes, so the filtering happens here
    let mut out: Vec<ObjectMeta> = Vec::new();
    for f in self.query("trashed = false")? {
      if f.name.starts_with(prefix)
        && validate_key(&f.name).is_ok()
        && !out.iter().any(|m| m.key == f.name)
      {
        out.push(f.into_meta());
      }
    }
    out.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(out)
  }

  fn stat(&self, key: &str) -> Result<Option<ObjectMeta>> {
    validate_key(key)?;
    Ok(self.find(key)?.into_iter().next().map(DriveFile::into_meta))
  }

  fn get(&self, key: &str) -> Result<Vec<u8>> {
    validate_key(key)?;
    let file = self
      .find(key)?
      .into_iter()
      .next()
      .ok_or_else(|| Error::NotFound(key.to_string()))?;
    let res = self
      .http
      .call(
        |a| {
          a.get(&format!("{API}/files/{}", file.id))
            .query("alt", "media")
        },
        Body::Empty,
      )
      .map_err(|e| http::not_found(e, key))?;
    http::read_body(res)
  }

  fn put(&self, key: &str, data: &[u8]) -> Result<ObjectMeta> {
    validate_key(key)?;
    check_size(data.len())?;
    let res = match self.find(key)?.into_iter().next() {
      Some(file) => self.http.call(
        |a| {
          a.request("PATCH", &format!("{UPLOAD}/files/{}", file.id))
            .query("uploadType", "media")
            .query("fields", FILE_FIELDS)
        },
        Body::Bytes {
          content_type: OCTET,
          data,
        },
      )?,
      None => {
        let meta = serde_json::json!({ "name": key, "parents": ["appDataFolder"] }).to_string();
        let (content_type, body) = multipart(&meta, data);
        self.http.call(
          |a| {
            a.post(&format!("{UPLOAD}/files"))
              .query("uploadType", "multipart")
              .query("fields", FILE_FIELDS)
          },
          Body::Bytes {
            content_type: &content_type,
            data: &body,
          },
        )?
      }
    };
    Ok(http::json::<DriveFile>(res)?.into_meta())
  }

  fn delete(&self, key: &str) -> Result<()> {
    validate_key(key)?;
    for file in self.find(key)? {
      match self.http.call(
        |a| a.delete(&format!("{API}/files/{}", file.id)),
        Body::Empty,
      ) {
        Ok(_) | Err(Error::Http { status: 404, .. }) => {}
        Err(e) => return Err(e),
      }
    }
    Ok(())
  }
}

/// Escapes a value for a single quoted string in the Drive query language.
fn escape(s: &str) -> String {
  s.replace('\\', "\\\\").replace('\'', "\\'")
}

// Metadata and content go into one request, so a create can never leave an empty file behind.
// The boundary is checked against the content, since the content is arbitrary bytes.
fn multipart(meta: &str, data: &[u8]) -> (String, Vec<u8>) {
  let mut n = 0u32;
  let boundary = loop {
    let candidate = format!("cloud-sync-{n}");
    if !data
      .windows(candidate.len())
      .any(|w| w == candidate.as_bytes())
    {
      break candidate;
    }
    n += 1;
  };
  let mut body = Vec::with_capacity(data.len() + meta.len() + 200);
  body.extend_from_slice(
    format!(
      "--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{meta}\r\n\
       --{boundary}\r\nContent-Type: {OCTET}\r\n\r\n"
    )
    .as_bytes(),
  );
  body.extend_from_slice(data);
  body.extend_from_slice(format!("\r\n--{boundary}--").as_bytes());
  (format!("multipart/related; boundary={boundary}"), body)
}

pub(crate) fn open(settings: Settings) -> Result<Box<dyn CloudStore>> {
  let tokens: Arc<dyn TokenProvider> = settings.tokens.ok_or(Error::MissingSetting("tokens"))?;
  Ok(Box::new(GoogleDriveStore {
    http: Http::new(tokens),
  }))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn escapes_query_values() {
    assert_eq!(escape(r"a'b\c"), r"a\'b\\c");
  }

  #[test]
  fn multipart_boundary_avoids_content() {
    let (ct, body) = multipart("{}", b"xx cloud-sync-0 yy");
    assert!(ct.ends_with("boundary=cloud-sync-1"));
    assert!(body.starts_with(b"--cloud-sync-1\r\n"));
    assert!(body.ends_with(b"\r\n--cloud-sync-1--"));
  }

  #[test]
  fn maps_file_to_meta() {
    let f: DriveFile = serde_json::from_str(
      r#"{"id":"1","name":"wal/a","size":"12","modifiedTime":"1970-01-01T00:00:02Z","md5Checksum":"abc","version":"7"}"#,
    )
    .unwrap();
    let m = f.into_meta();
    assert_eq!(
      (m.key.as_str(), m.size, m.modified_ms, m.revision.as_str()),
      ("wal/a", 12, Some(2000), "abc")
    );
  }
}
