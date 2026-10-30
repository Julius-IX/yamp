//! Big objects, such as database snapshots.
//!
//! A backend object is capped at `MAX_OBJECT_SIZE`, so a blob is cut into chunks. The
//! chunks of one write live in their own generation directory and the manifest that
//! points at it is written last. A reader sees a blob only once the manifest exists,
//! so an interrupted upload leaves the old version intact. Replacing a blob switches
//! the manifest to the new generation first and removes the old chunks after.
//! Reads check size and SHA-256, so a torn or damaged blob is an error and never wrong data.

use std::io::{self, Read, Write};

use cloud_sync::{Error as StoreError, MAX_OBJECT_SIZE};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{stamp, Dir, Error, Result};

const MANIFEST: &str = "manifest.json";
const MANIFEST_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobInfo {
  /// Filled in on read, it is not part of the stored manifest.
  #[serde(skip)]
  pub name: String,
  pub size: u64,
  pub chunks: u32,
  /// Lowercase hex.
  pub sha256: String,
  pub created_ms: i64,
  version: u32,
  pub(crate) generation: String,
}

/// Blobs in a directory, one subdirectory per blob. The directory should hold blobs
/// only, since listing them reads past every chunk.
///
/// Like everything here a blob has a single writer. A reader that races a replacement
/// can hit a removed chunk and gets `Corrupt`, a retry then sees the new version.
#[derive(Clone)]
pub struct Blobs {
  dir: Dir,
  chunk_size: usize,
}

impl Blobs {
  pub fn new(dir: Dir) -> Self {
    Self {
      dir,
      chunk_size: MAX_OBJECT_SIZE,
    }
  }

  /// Smaller chunks suit slow or flaky connections. The default is the largest allowed.
  pub fn with_chunk_size(mut self, bytes: usize) -> Self {
    self.chunk_size = bytes.clamp(1, MAX_OBJECT_SIZE);
    self
  }

  /// Creates the blob or replaces it, reading from `reader` one chunk at a time so
  /// a large file never has to fit in memory.
  pub fn put_reader(&self, name: &str, mut reader: impl Read) -> Result<BlobInfo> {
    let dir = self.blob_dir(name)?;
    let generation = format!("g{}", stamp::now());
    let chunk_dir = dir.dir(&generation)?;

    let mut buf = vec![0u8; self.chunk_size];
    let mut hasher = Sha256::new();
    let (mut size, mut chunks) = (0u64, 0u32);
    let uploaded = (|| -> Result<()> {
      loop {
        let n = fill(&mut reader, &mut buf)?;
        if n == 0 {
          return Ok(());
        }
        hasher.update(&buf[..n]);
        chunk_dir.put_bytes(&chunk_name(chunks), &buf[..n])?;
        size += n as u64;
        chunks += 1;
        // a short chunk can only be the last one
        if n < buf.len() {
          return Ok(());
        }
      }
    })();

    let info = BlobInfo {
      name: name.to_string(),
      size,
      chunks,
      sha256: hex(&hasher.finalize()),
      created_ms: now_ms(),
      version: MANIFEST_VERSION,
      generation: generation.clone(),
    };
    let committed = uploaded.and_then(|_| dir.put_json(MANIFEST, &info).map(|_| ()));
    if let Err(e) = committed {
      log::warn!("upload of blob {name} failed after {chunks} chunks: {e}");
      // best effort, the next write sweeps what is left
      let _ = chunk_dir.delete_dir();
      return Err(e);
    }
    sweep(&dir, &generation);
    log::info!("stored blob {name}: {size} bytes in {chunks} chunks");
    Ok(info)
  }

  /// Creates the blob or replaces it.
  pub fn put(&self, name: &str, data: &[u8]) -> Result<BlobInfo> {
    self.put_reader(name, data)
  }

  /// Like [`put_reader`](Self::put_reader), but fails with `AlreadyExists` if the name is taken.
  pub fn insert_reader(&self, name: &str, reader: impl Read) -> Result<BlobInfo> {
    if self.info(name)?.is_some() {
      return Err(self.dir.already_exists(name));
    }
    self.put_reader(name, reader)
  }

  pub fn insert(&self, name: &str, data: &[u8]) -> Result<BlobInfo> {
    self.insert_reader(name, data)
  }

  pub fn info(&self, name: &str) -> Result<Option<BlobInfo>> {
    let Some(mut info) = self.blob_dir(name)?.get_json_opt::<BlobInfo>(MANIFEST)? else {
      return Ok(None);
    };
    if info.version != MANIFEST_VERSION {
      return Err(Error::Corrupt(format!(
        "{name}: unsupported manifest version {}",
        info.version
      )));
    }
    info.name = name.to_string();
    Ok(Some(info))
  }

  /// Streams the blob into `writer`. The checksum is only known after the last chunk,
  /// so a failed check leaves the data already written. Writing to a temporary file
  /// and renaming it on success keeps that from mattering.
  pub fn get_to_writer(&self, name: &str, mut writer: impl Write) -> Result<BlobInfo> {
    let info = self
      .info(name)?
      .ok_or_else(|| StoreError::NotFound(self.dir.path().to_string() + "/" + name))?;
    let chunk_dir = self.blob_dir(name)?.dir(&info.generation)?;

    let mut hasher = Sha256::new();
    let mut size = 0u64;
    for i in 0..info.chunks {
      let data = chunk_dir.get_bytes(&chunk_name(i)).map_err(|e| {
        if e.is_not_found() {
          log::warn!("blob {name} lost chunk {i}");
          Error::Corrupt(format!("{name}: chunk {i} is missing"))
        } else {
          e
        }
      })?;
      hasher.update(&data);
      size += data.len() as u64;
      writer.write_all(&data)?;
    }
    if size != info.size || hex(&hasher.finalize()) != info.sha256 {
      log::warn!(
        "blob {name} failed verification, {size} of {} bytes read",
        info.size
      );
      return Err(Error::Corrupt(format!(
        "{name}: size or checksum does not match the manifest"
      )));
    }
    Ok(info)
  }

  /// The whole blob in memory, verified before it is returned.
  pub fn get(&self, name: &str) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    self.get_to_writer(name, &mut out)?;
    Ok(out)
  }

  /// Removes the blob. The manifest goes first, so the blob disappears for readers
  /// at once and an interrupted delete only leaves unreachable chunks behind.
  pub fn delete(&self, name: &str) -> Result<()> {
    let dir = self.blob_dir(name)?;
    dir.delete(MANIFEST)?;
    if !dir.list_all()?.is_empty() {
      dir.delete_dir()?;
    }
    Ok(())
  }

  /// Names of all blobs, sorted. Blobs named with [`stamp::now`] sort by age.
  pub fn list_names(&self) -> Result<Vec<String>> {
    let manifest = format!("/{MANIFEST}");
    Ok(
      self
        .dir
        .list_all()?
        .into_iter()
        .filter_map(|m| {
          m.key
            .strip_suffix(&manifest)
            .filter(|n| !n.contains('/'))
            .map(str::to_string)
        })
        .collect(),
    )
  }

  pub fn list(&self) -> Result<Vec<BlobInfo>> {
    let mut out = Vec::new();
    for name in self.list_names()? {
      out.extend(self.info(&name)?);
    }
    Ok(out)
  }

  fn blob_dir(&self, name: &str) -> Result<Dir> {
    if name.contains('/') {
      return Err(
        StoreError::InvalidKey(format!("{name:?} (blob names are a single segment)")).into(),
      );
    }
    self.dir.dir(name)
  }
}

/// Removes everything in a blob directory except the manifest and the live generation.
/// Failures are ignored on purpose, a leftover chunk set is unreachable and harmless.
fn sweep(dir: &Dir, keep: &str) {
  let items = match dir.list_all() {
    Ok(items) => items,
    Err(e) => return log::debug!("could not list {} for cleanup: {e}", dir.path()),
  };
  let live = format!("{keep}/");
  for m in items {
    if m.key != MANIFEST && !m.key.starts_with(&live) {
      if let Err(e) = dir.delete(&m.key) {
        log::debug!("could not remove stale {}: {e}", m.key);
      }
    }
  }
}

fn chunk_name(index: u32) -> String {
  format!("{index:08}")
}

/// Reads until the buffer is full or the input ends, since `read` alone may return short.
fn fill<R: Read>(reader: &mut R, buf: &mut [u8]) -> io::Result<usize> {
  let mut filled = 0;
  while filled < buf.len() {
    match reader.read(&mut buf[filled..]) {
      Ok(0) => break,
      Ok(n) => filled += n,
      Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
      Err(e) => return Err(e),
    }
  }
  Ok(filled)
}

fn hex(bytes: &[u8]) -> String {
  bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn now_ms() -> i64 {
  std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map_or(0, |d| d.as_millis() as i64)
}
