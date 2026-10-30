use serde::{de::DeserializeOwned, Serialize};

use crate::{stamp, Dir, Result};

const EXT: &str = ".json";

/// An append only sequence of JSON entries in a directory, ordered by time.
///
/// Each entry is a file named by [`stamp::now`], so listing the directory returns the
/// entries in order and a reader can resume from the last name it handled, its cursor.
/// This is the shape for shipping changes between devices. What an entry contains is
/// up to the caller. Only one device should append to a log, which means one log per device.
#[derive(Clone)]
pub struct Log {
  dir: Dir,
}

impl Log {
  pub fn new(dir: Dir) -> Self {
    Self { dir }
  }

  pub fn dir(&self) -> &Dir {
    &self.dir
  }

  /// Appends an entry and returns its name.
  pub fn append<T: Serialize>(&self, value: &T) -> Result<String> {
    let name = format!("{}{EXT}", stamp::now());
    self.dir.put_json(&name, value)?;
    Ok(name)
  }

  /// Names of entries after the cursor, oldest first. Without a cursor, all of them.
  pub fn names_after(&self, cursor: Option<&str>, limit: Option<usize>) -> Result<Vec<String>> {
    let names = self
      .dir
      .list()?
      .into_iter()
      .map(|m| m.key)
      .filter(|name| entry_time_ms(name).is_some())
      .filter(|name| !cursor.is_some_and(|c| name.as_str() <= c));
    Ok(names.take(limit.unwrap_or(usize::MAX)).collect())
  }

  pub fn read<T: DeserializeOwned>(&self, name: &str) -> Result<T> {
    self.dir.get_json(name)
  }

  /// Entries after the cursor with their names, oldest first.
  /// The name of the last one is the cursor for the next call.
  pub fn read_after<T: DeserializeOwned>(
    &self,
    cursor: Option<&str>,
    limit: Option<usize>,
  ) -> Result<Vec<(String, T)>> {
    let mut out = Vec::new();
    for name in self.names_after(cursor, limit)? {
      let value = self.read(&name)?;
      out.push((name, value));
    }
    Ok(out)
  }

  pub fn latest(&self) -> Result<Option<String>> {
    Ok(self.names_after(None, None)?.pop())
  }

  /// Deletes the entries up to and including the cursor and returns how many.
  /// Used once every reader has moved past them, so the log does not grow forever.
  pub fn trim_through(&self, cursor: &str) -> Result<usize> {
    let doomed: Vec<String> = self
      .dir
      .list()?
      .into_iter()
      .map(|m| m.key)
      .filter(|n| entry_time_ms(n).is_some() && n.as_str() <= cursor)
      .collect();
    for name in &doomed {
      self.dir.delete(name)?;
    }
    log::debug!("trimmed {} entries of {}", doomed.len(), self.dir.path());
    Ok(doomed.len())
  }
}

/// When an entry was appended, `None` if the name is not a log entry.
pub fn entry_time_ms(name: &str) -> Option<i64> {
  stamp::parse_ms(name.strip_suffix(EXT)?)
}
