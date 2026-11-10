//! Reads the Discord application id and passes it to the compiler as an environment variable,
//! which is how `lib.rs` receives it through `option_env!`.
//!
//! Cargo can do this itself with the `[env]` table of a `.cargo/config.toml`, but only for a
//! config that is found from the folder cargo is started in, and the mobile builds start it from
//! places that differ per tool. A build script finds its files from the folder of this crate,
//! wherever cargo was started. The sources, the first one that has a non empty value wins:
//!
//! 1. the environment of the build, which is what a CI build sets,
//! 2. `config.toml` in this folder,
//! 3. the `[env]` table of `src-tauri/.cargo/config.toml`.
//!
//! The same lookup is in `cloud-auth`. It is copied and not shared, since a build script cannot
//! depend on the code of another crate without making it a build dependency of both.

use std::{
  collections::BTreeMap,
  env, fs,
  path::{Path, PathBuf},
};

const NAME: &str = "YAMP_DISCORD_APP_ID";

/// The `KEY = "value"` lines of a config file. That is all the syntax the files use. With
/// `flat` the keys may also stand above any table header.
fn read(path: &Path, flat: bool) -> BTreeMap<String, String> {
  let mut out = BTreeMap::new();
  let Ok(text) = fs::read_to_string(path) else { return out };
  let mut in_env = flat;
  for line in text.lines().map(str::trim) {
    if line.is_empty() || line.starts_with('#') {
      continue;
    }
    if line.starts_with('[') {
      in_env = line == "[env]";
      continue;
    }
    let Some((key, value)) = line.split_once('=') else { continue };
    let value = value.trim();
    let value = ['"', '\'']
      .iter()
      .find_map(|q| value.strip_prefix(*q).and_then(|v| v.split_once(*q)).map(|(v, _)| v))
      .unwrap_or("");
    if in_env && !value.is_empty() {
      out.insert(key.trim().to_owned(), value.to_owned());
    }
  }
  out
}

fn main() {
  let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("set by cargo"));
  let local = manifest.join("config.toml");
  let cargo = manifest.join("../../.cargo/config.toml");
  println!("cargo:rerun-if-changed=build.rs");
  println!("cargo:rerun-if-changed={}", local.display());
  println!("cargo:rerun-if-changed={}", cargo.display());
  println!("cargo:rerun-if-env-changed={NAME}");

  // an empty variable counts as not set, so a blank line in one file does not hide a value in another
  if env::var(NAME).is_ok_and(|v| !v.is_empty()) {
    return;
  }
  let (from_local, from_cargo) = (read(&local, true), read(&cargo, false));
  if let Some(value) = from_local.get(NAME).or_else(|| from_cargo.get(NAME)) {
    println!("cargo:rustc-env={NAME}={value}");
  }
}
