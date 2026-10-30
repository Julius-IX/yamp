use crate::{Error, Result};

const MAX_KEY_LEN: usize = 400;
const MAX_SEGMENT_LEN: usize = 255;
// Reserved on Windows, which is the strictest of the three services.
const FORBIDDEN: &[char] = &['\\', ':', '*', '?', '"', '<', '>', '|'];

/// Checks a key against the rules that hold on every backend.
///
/// A key is `/` separated segments. Segments may not start with a dot, because the
/// local backend uses dot files for temporary files and iCloud placeholders. Keys
/// that differ only in case collide on OneDrive and macOS, so lowercase keys are advised.
pub fn validate_key(key: &str) -> Result<()> {
  if key.is_empty() {
    return invalid(key, "empty");
  }
  if key.len() > MAX_KEY_LEN {
    return invalid(key, "too long");
  }
  key.split('/').try_for_each(|seg| check_segment(seg, key))
}

/// Checks a listing prefix. Unlike a key it may be empty, end in `/` or end in a partial segment.
pub fn validate_prefix(prefix: &str) -> Result<()> {
  if prefix.len() > MAX_KEY_LEN {
    return invalid(prefix, "too long");
  }
  let count = prefix.split('/').count();
  for (i, seg) in prefix.split('/').enumerate() {
    if i + 1 == count && seg.is_empty() {
      continue;
    }
    check_segment(seg, prefix)?;
  }
  Ok(())
}

/// The directory part of a prefix, used to avoid listing folders that cannot match.
pub(crate) fn split_prefix_dir(prefix: &str) -> &str {
  prefix.rfind('/').map_or("", |i| &prefix[..i])
}

fn check_segment(seg: &str, whole: &str) -> Result<()> {
  if seg.is_empty() {
    return invalid(whole, "empty segment");
  }
  if seg.len() > MAX_SEGMENT_LEN {
    return invalid(whole, "segment too long");
  }
  if seg.starts_with('.') {
    return invalid(whole, "segment starts with a dot");
  }
  if seg.ends_with(' ') || seg.ends_with('.') {
    return invalid(whole, "segment ends with a space or dot");
  }
  if seg
    .chars()
    .any(|c| c.is_control() || FORBIDDEN.contains(&c))
  {
    return invalid(whole, "forbidden character");
  }
  Ok(())
}

fn invalid(key: &str, why: &str) -> Result<()> {
  Err(Error::InvalidKey(format!("{key:?} ({why})")))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn accepts_plain_and_nested_keys() {
    for k in ["a", "wal/dev1/0001.seg", "a b/c-d_e"] {
      assert!(validate_key(k).is_ok(), "{k}");
    }
  }

  #[test]
  fn rejects_bad_keys() {
    for k in [
      "", "/a", "a/", "a//b", ".hidden", "a/.b", "a/../b", "a\\b", "a:b", "a.", "a\nb",
    ] {
      assert!(validate_key(k).is_err(), "{k:?}");
    }
  }

  #[test]
  fn prefix_allows_partial_tail() {
    for p in ["", "wal/", "wal/dev", "wal/dev1/0001"] {
      assert!(validate_prefix(p).is_ok(), "{p}");
    }
    for p in ["/", "/wal", "wal//x", ".wal"] {
      assert!(validate_prefix(p).is_err(), "{p}");
    }
  }

  #[test]
  fn dir_part_of_prefix() {
    assert_eq!(split_prefix_dir(""), "");
    assert_eq!(split_prefix_dir("wal"), "");
    assert_eq!(split_prefix_dir("wal/"), "wal");
    assert_eq!(split_prefix_dir("wal/dev1/00"), "wal/dev1");
  }
}
