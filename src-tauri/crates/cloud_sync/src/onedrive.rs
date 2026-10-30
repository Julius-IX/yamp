//! OneDrive backend on the `approot` special folder through Microsoft Graph.
//!
//! The folder is private to the app. Keys map to real paths below it, and a path
//! based upload creates missing folders on its own. Graph has no recursive listing
//! for this folder, so [`list`](CloudStore::list) walks the folders that can match.

use std::sync::Arc;

use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use serde::Deserialize;

use crate::{
  http::{self, Body, Http},
  key::{split_prefix_dir, validate_key, validate_prefix},
  store::check_size,
  CloudStore, Error, ObjectMeta, Result, Settings, TokenProvider,
};

const ROOT: &str = "https://graph.microsoft.com/v1.0/me/drive/special/approot";
const SELECT: &str = "name,size,cTag,eTag,lastModifiedDateTime,folder";
const SEGMENT: &AsciiSet = &NON_ALPHANUMERIC
  .remove(b'-')
  .remove(b'_')
  .remove(b'.')
  .remove(b'~');

pub(crate) struct OneDriveStore {
  http: Http,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Item {
  name: String,
  size: Option<u64>,
  last_modified_date_time: Option<String>,
  c_tag: Option<String>,
  e_tag: Option<String>,
  // only present on folders
  folder: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct Children {
  value: Vec<Item>,
  #[serde(rename = "@odata.nextLink")]
  next_link: Option<String>,
}

impl Item {
  fn into_meta(self, key: String) -> ObjectMeta {
    ObjectMeta {
      key,
      size: self.size.unwrap_or(0),
      modified_ms: self
        .last_modified_date_time
        .as_deref()
        .and_then(http::rfc3339_ms),
      // cTag moves with the content only, eTag also with metadata
      revision: self.c_tag.or(self.e_tag).unwrap_or_default(),
    }
  }
}

impl OneDriveStore {
  /// Direct children of a folder, or `None` when the folder does not exist.
  fn children(&self, dir: &str) -> Result<Option<Vec<Item>>> {
    let mut items = Vec::new();
    let mut next: Option<String> = None;
    loop {
      let res = match &next {
        None => self.http.call(
          |a| {
            a.get(&children_url(dir))
              .query("$select", SELECT)
              .query("$top", "200")
          },
          Body::Empty,
        ),
        Some(link) => self.http.call(|a| a.get(link), Body::Empty),
      };
      let res = match res {
        Ok(res) => res,
        Err(Error::Http { status: 404, .. }) => return Ok(None),
        Err(e) => return Err(e),
      };
      let page: Children = http::json(res)?;
      items.extend(page.value);
      match page.next_link {
        Some(link) => next = Some(link),
        None => return Ok(Some(items)),
      }
    }
  }
}

impl CloudStore for OneDriveStore {
  fn list(&self, prefix: &str) -> Result<Vec<ObjectMeta>> {
    validate_prefix(prefix)?;
    let mut out = Vec::new();
    let mut stack = vec![split_prefix_dir(prefix).to_string()];
    while let Some(dir) = stack.pop() {
      let Some(items) = self.children(&dir)? else {
        continue;
      };
      for item in items {
        let path = if dir.is_empty() {
          item.name.clone()
        } else {
          format!("{dir}/{}", item.name)
        };
        if item.folder.is_some() {
          let slashed = format!("{path}/");
          if slashed.starts_with(prefix) || prefix.starts_with(&slashed) {
            stack.push(path);
          }
        } else if path.starts_with(prefix) && validate_key(&path).is_ok() {
          out.push(item.into_meta(path));
        }
      }
    }
    out.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(out)
  }

  fn stat(&self, key: &str) -> Result<Option<ObjectMeta>> {
    validate_key(key)?;
    match self.http.call(
      |a| a.get(&item_url(key)).query("$select", SELECT),
      Body::Empty,
    ) {
      Ok(res) => {
        let item: Item = http::json(res)?;
        Ok(if item.folder.is_some() {
          None
        } else {
          Some(item.into_meta(key.to_string()))
        })
      }
      Err(Error::Http { status: 404, .. }) => Ok(None),
      Err(e) => Err(e),
    }
  }

  fn get(&self, key: &str) -> Result<Vec<u8>> {
    validate_key(key)?;
    let res = self
      .http
      .call_no_redirect(
        |a| a.get(&format!("{}/content", item_url(key))),
        Body::Empty,
      )
      .map_err(|e| http::not_found(e, key))?;
    if (300..400).contains(&res.status()) {
      let link = res
        .header("Location")
        .ok_or_else(|| Error::Protocol("redirect without a location".into()))?
        .to_string();
      return http::read_body(self.http.plain_get(&link)?);
    }
    http::read_body(res)
  }

  fn put(&self, key: &str, data: &[u8]) -> Result<ObjectMeta> {
    validate_key(key)?;
    check_size(data.len())?;
    let res = self.http.call(
      |a| {
        a.put(&format!("{}/content", item_url(key)))
          .query("@microsoft.graph.conflictBehavior", "replace")
          .query("$select", SELECT)
      },
      Body::Bytes {
        content_type: "application/octet-stream",
        data,
      },
    )?;
    Ok(http::json::<Item>(res)?.into_meta(key.to_string()))
  }

  fn delete(&self, key: &str) -> Result<()> {
    validate_key(key)?;
    match self.http.call(|a| a.delete(&item_url(key)), Body::Empty) {
      Ok(_) | Err(Error::Http { status: 404, .. }) => Ok(()),
      Err(e) => Err(e),
    }
  }
}

fn encode_path(path: &str) -> String {
  path
    .split('/')
    .map(|s| utf8_percent_encode(s, SEGMENT).to_string())
    .collect::<Vec<_>>()
    .join("/")
}

fn item_url(key: &str) -> String {
  format!("{ROOT}:/{}:", encode_path(key))
}

fn children_url(dir: &str) -> String {
  if dir.is_empty() {
    format!("{ROOT}/children")
  } else {
    format!("{}/children", item_url(dir))
  }
}

pub(crate) fn open(settings: Settings) -> Result<Box<dyn CloudStore>> {
  let tokens: Arc<dyn TokenProvider> = settings.tokens.ok_or(Error::MissingSetting("tokens"))?;
  Ok(Box::new(OneDriveStore {
    http: Http::new(tokens),
  }))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn builds_path_urls() {
    assert_eq!(
      item_url("wal/dev 1/a#b"),
      format!("{ROOT}:/wal/dev%201/a%23b:")
    );
    assert_eq!(children_url(""), format!("{ROOT}/children"));
    assert_eq!(children_url("wal"), format!("{ROOT}:/wal:/children"));
  }

  #[test]
  fn maps_item_to_meta() {
    let item: Item = serde_json::from_str(
      r#"{"name":"a","size":3,"cTag":"c1","eTag":"e1","lastModifiedDateTime":"1970-01-01T00:00:03Z"}"#,
    )
    .unwrap();
    assert!(item.folder.is_none());
    let m = item.into_meta("wal/a".into());
    assert_eq!(
      (m.size, m.modified_ms, m.revision.as_str()),
      (3, Some(3000), "c1")
    );
  }
}
