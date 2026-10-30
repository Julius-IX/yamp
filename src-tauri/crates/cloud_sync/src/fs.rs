//! Store backed by a plain directory.
//!
//! This is the iCloud backend, since the OS syncs the app's ubiquity container like
//! any folder. It also fits a folder kept in sync by a desktop client.

use std::{
  fs, io,
  path::{Path, PathBuf},
  sync::atomic::{AtomicU64, Ordering},
  time::UNIX_EPOCH,
};

use crate::{
  key::{split_prefix_dir, validate_key, validate_prefix},
  store::check_size,
  CloudStore, Error, ObjectMeta, Result,
};

// Dot prefixed so that a half written file is never listed as an object.
const TMP_PREFIX: &str = ".cloud-sync-tmp-";
static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct LocalDirStore {
  root: PathBuf,
}

impl LocalDirStore {
  pub fn new(root: impl Into<PathBuf>) -> Result<Self> {
    let root = root.into();
    fs::create_dir_all(&root)?;
    Ok(Self { root })
  }

  fn path(&self, key: &str) -> PathBuf {
    key.split('/').fold(self.root.clone(), |p, seg| p.join(seg))
  }

  fn walk(&self, dir: &Path, rel: &str, out: &mut Vec<ObjectMeta>) -> Result<()> {
    let entries = match fs::read_dir(dir) {
      Ok(e) => e,
      Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
      Err(e) => return Err(e.into()),
    };
    for entry in entries {
      let entry = entry?;
      let Ok(name) = entry.file_name().into_string() else {
        continue;
      };
      let md = entry.metadata()?;
      if md.is_dir() {
        self.walk(&entry.path(), &format!("{rel}{name}/"), out)?;
      } else if let Some(real) = placeholder_target(&name) {
        out.push(placeholder_meta(format!("{rel}{real}"), &md));
      } else if !name.starts_with('.') {
        out.push(meta(format!("{rel}{name}"), &md));
      }
    }
    Ok(())
  }
}

impl CloudStore for LocalDirStore {
  fn list(&self, prefix: &str) -> Result<Vec<ObjectMeta>> {
    validate_prefix(prefix)?;
    let dir = split_prefix_dir(prefix);
    let rel = if dir.is_empty() {
      String::new()
    } else {
      format!("{dir}/")
    };
    let start = if dir.is_empty() {
      self.root.clone()
    } else {
      self.path(dir)
    };

    let mut out = Vec::new();
    self.walk(&start, &rel, &mut out)?;
    out.retain(|m| m.key.starts_with(prefix));
    out.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(out)
  }

  fn stat(&self, key: &str) -> Result<Option<ObjectMeta>> {
    validate_key(key)?;
    let path = self.path(key);
    match fs::metadata(&path) {
      Ok(md) if md.is_file() => return Ok(Some(meta(key.to_string(), &md))),
      Ok(_) => return Ok(None),
      Err(e) if e.kind() == io::ErrorKind::NotFound => {}
      Err(e) => return Err(e.into()),
    }
    match fs::metadata(placeholder_path(&path)) {
      Ok(md) => Ok(Some(placeholder_meta(key.to_string(), &md))),
      Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
      Err(e) => Err(e.into()),
    }
  }

  fn get(&self, key: &str) -> Result<Vec<u8>> {
    validate_key(key)?;
    let path = self.path(key);
    match fs::read(&path) {
      Ok(data) => Ok(data),
      Err(e) if e.kind() == io::ErrorKind::NotFound => {
        if placeholder_path(&path).exists() {
          log::debug!("{key} is an icloud placeholder, the download has not happened yet");
          Err(Error::NotDownloaded(key.to_string()))
        } else {
          Err(Error::NotFound(key.to_string()))
        }
      }
      Err(e) => Err(e.into()),
    }
  }

  fn put(&self, key: &str, data: &[u8]) -> Result<ObjectMeta> {
    validate_key(key)?;
    check_size(data.len())?;
    let path = self.path(key);
    let parent = path.parent().unwrap_or(&self.root);
    fs::create_dir_all(parent)?;

    // write then rename, so a reader or the sync daemon never sees a partial object
    let tmp = parent.join(format!(
      "{TMP_PREFIX}{}-{}",
      std::process::id(),
      TMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let written = fs::write(&tmp, data)
      .and_then(|_| fs::File::open(&tmp)?.sync_all())
      .and_then(|_| fs::rename(&tmp, &path));
    if let Err(e) = written {
      let _ = fs::remove_file(&tmp);
      return Err(e.into());
    }
    Ok(meta(key.to_string(), &fs::metadata(&path)?))
  }

  fn delete(&self, key: &str) -> Result<()> {
    validate_key(key)?;
    let path = self.path(key);
    for p in [placeholder_path(&path), path] {
      match fs::remove_file(&p) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
      }
    }
    Ok(())
  }
}

// iCloud replaces a file that is not downloaded with an empty `.<name>.icloud` stub.
// It still counts as an object, only reading it has to wait for the download.
fn placeholder_target(name: &str) -> Option<&str> {
  name
    .strip_prefix('.')?
    .strip_suffix(".icloud")
    .filter(|n| !n.is_empty())
}

fn placeholder_path(path: &Path) -> PathBuf {
  let name = path
    .file_name()
    .map(|n| n.to_string_lossy())
    .unwrap_or_default();
  path.with_file_name(format!(".{name}.icloud"))
}

fn placeholder_meta(key: String, md: &fs::Metadata) -> ObjectMeta {
  // the stub carries no size or content version, so the real values are unknown
  ObjectMeta {
    size: 0,
    revision: String::from("placeholder"),
    ..meta(key, md)
  }
}

fn meta(key: String, md: &fs::Metadata) -> ObjectMeta {
  let modified = md
    .modified()
    .ok()
    .and_then(|t| t.duration_since(UNIX_EPOCH).ok());
  ObjectMeta {
    key,
    size: md.len(),
    modified_ms: modified.map(|d| d.as_millis() as i64),
    revision: format!("{}-{}", modified.map_or(0, |d| d.as_nanos()), md.len()),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn store(name: &str) -> (LocalDirStore, PathBuf) {
    let dir = std::env::temp_dir().join(format!("cloud-sync-test-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    (LocalDirStore::new(&dir).unwrap(), dir)
  }

  #[test]
  fn put_get_stat_delete() {
    let (s, dir) = store("roundtrip");
    let m = s.put("wal/dev1/0001", b"hello").unwrap();
    assert_eq!((m.key.as_str(), m.size), ("wal/dev1/0001", 5));
    assert_eq!(s.get("wal/dev1/0001").unwrap(), b"hello");
    assert_eq!(
      s.stat("wal/dev1/0001").unwrap().unwrap().revision,
      m.revision
    );

    s.put("wal/dev1/0001", b"hello world").unwrap();
    assert_eq!(s.get("wal/dev1/0001").unwrap(), b"hello world");

    s.delete("wal/dev1/0001").unwrap();
    s.delete("wal/dev1/0001").unwrap();
    assert!(s.stat("wal/dev1/0001").unwrap().is_none());
    assert!(matches!(s.get("wal/dev1/0001"), Err(Error::NotFound(_))));
    let _ = fs::remove_dir_all(dir);
  }

  #[test]
  fn list_filters_by_prefix_and_sorts() {
    let (s, dir) = store("list");
    for k in ["wal/dev2/0001", "wal/dev1/0002", "wal/dev1/0001", "snap/x"] {
      s.put(k, b"x").unwrap();
    }
    let keys = |p: &str| {
      s.list(p)
        .unwrap()
        .into_iter()
        .map(|m| m.key)
        .collect::<Vec<_>>()
    };
    assert_eq!(keys("wal/dev1/"), ["wal/dev1/0001", "wal/dev1/0002"]);
    assert_eq!(
      keys("wal/dev"),
      ["wal/dev1/0001", "wal/dev1/0002", "wal/dev2/0001"]
    );
    assert_eq!(keys("").len(), 4);
    assert!(keys("nope/").is_empty());
    let _ = fs::remove_dir_all(dir);
  }

  #[test]
  fn temp_and_hidden_files_are_not_objects() {
    let (s, dir) = store("hidden");
    s.put("a", b"1").unwrap();
    fs::write(dir.join(format!("{TMP_PREFIX}1-1")), b"x").unwrap();
    fs::write(dir.join(".DS_Store"), b"x").unwrap();
    assert_eq!(s.list("").unwrap().len(), 1);
    let _ = fs::remove_dir_all(dir);
  }

  #[test]
  fn icloud_placeholder_is_listed_but_not_readable() {
    let (s, dir) = store("placeholder");
    fs::create_dir_all(dir.join("wal")).unwrap();
    fs::write(dir.join("wal").join(".0001.icloud"), b"").unwrap();
    let list = s.list("wal/").unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].key, "wal/0001");
    assert!(s.stat("wal/0001").unwrap().is_some());
    assert!(matches!(s.get("wal/0001"), Err(Error::NotDownloaded(_))));
    s.delete("wal/0001").unwrap();
    assert!(s.stat("wal/0001").unwrap().is_none());
    let _ = fs::remove_dir_all(dir);
  }

  #[test]
  fn rejects_bad_keys_and_big_objects() {
    let (s, dir) = store("limits");
    assert!(matches!(s.put("../x", b""), Err(Error::InvalidKey(_))));
    let big = vec![0u8; crate::MAX_OBJECT_SIZE + 1];
    assert!(matches!(s.put("big", &big), Err(Error::TooLarge { .. })));
    let _ = fs::remove_dir_all(dir);
  }
}
