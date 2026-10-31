//! Desktop engine. yt-dlp is the standalone release binary, downloaded into an app owned
//! directory so no Python install is needed and updates never touch the system.
use super::engine::{YtDlpArgs, YtDlpEngine};
use crate::{
  error::{MediaError, Result},
  ProgressSink,
};
use async_trait::async_trait;
use sha2::{Digest, Sha256};
use std::{path::PathBuf, process::Stdio, time::Duration};
use tokio::{
  io::{AsyncBufReadExt, AsyncRead, BufReader},
  process::Command,
};

const BASE: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download";

/// Listings and metadata should finish quickly. A hung extractor would otherwise keep the
/// command pending forever. Downloads are exempt since their length depends on the file.
const QUERY_TIMEOUT: Duration = Duration::from_secs(90);

/// Marks the machine readable progress lines so they cannot be mistaken for yt-dlp's own output.
const PROGRESS_PREFIX: &str = "yamp-progress";
const PROGRESS_TEMPLATE: &str = "download:yamp-progress %(progress.downloaded_bytes)s %(progress.total_bytes)s %(progress.total_bytes_estimate)s";

#[cfg(all(windows, target_arch = "x86_64"))]
const ASSET: &str = "yt-dlp.exe";
#[cfg(all(windows, target_arch = "aarch64"))]
const ASSET: &str = "yt-dlp_arm64.exe";
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const ASSET: &str = "yt-dlp_linux";
#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
const ASSET: &str = "yt-dlp_linux_aarch64";
#[cfg(target_os = "macos")]
const ASSET: &str = "yt-dlp_macos";

// SECTION: process execution
pub struct DesktopEngine {
  dir: PathBuf,
}

impl DesktopEngine {
  /// `dir` is created on first install.
  pub fn new(dir: impl Into<PathBuf>) -> Self {
    Self { dir: dir.into() }
  }

  fn bin(&self) -> PathBuf {
    self.dir.join(ASSET)
  }

  async fn exec(&self, args: &YtDlpArgs, progress: Option<&ProgressSink>) -> Result<String> {
    let bin = self.bin();
    if !bin.exists() {
      return Err(MediaError::NotInstalled);
    }
    log::debug!("yt-dlp {}", args.target);
    let mut cmd = Command::new(bin);
    for o in &args.opts {
      cmd.arg(&o.k);
      if !o.v.is_empty() {
        cmd.arg(&o.v);
      }
    }
    if progress.is_some() {
      cmd.args(["--newline", "--progress", "--progress-template", PROGRESS_TEMPLATE]);
    }
    // the separator keeps the target from ever being parsed as a flag
    if !args.target.is_empty() {
      cmd.arg("--").arg(&args.target);
    }
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW

    let mut child = cmd.spawn()?;
    let stdout = child.stdout.take().ok_or_else(|| MediaError::Engine("no stdout".into()))?;
    let stderr = child.stderr.take().ok_or_else(|| MediaError::Engine("no stderr".into()))?;
    // both pipes are drained at once, a full pipe would otherwise stall the child
    let (out, err) = tokio::join!(drain(stdout, progress), drain(stderr, progress));
    let status = child.wait().await?;
    if status.success() {
      return Ok(out?);
    }
    let last = err?.lines().rev().find(|l| !l.trim().is_empty()).map(str::to_owned);
    let message =
      last.map(|l| l.trim_start_matches("ERROR: ").to_owned()).unwrap_or_else(|| "yt-dlp failed".into());
    log::debug!("yt-dlp exited with {status}: {message}");
    Err(MediaError::Engine(message))
  }
}

/// Collects everything except progress lines, which go to `progress` instead.
async fn drain(pipe: impl AsyncRead + Unpin, progress: Option<&ProgressSink>) -> std::io::Result<String> {
  let mut lines = BufReader::new(pipe).lines();
  let mut buf = String::new();
  while let Some(line) = lines.next_line().await? {
    match progress.and_then(|_| parse_progress(&line)) {
      Some(percent) => {
        if let Some(p) = progress {
          p(percent);
        }
      }
      None => {
        buf.push_str(&line);
        buf.push('\n');
      }
    }
  }
  Ok(buf)
}

/// `yamp-progress <downloaded> <total> <total estimate>`, each field `NA` when unknown.
fn parse_progress(line: &str) -> Option<f32> {
  let mut f = line.strip_prefix(PROGRESS_PREFIX)?.split_whitespace();
  let done: f64 = f.next()?.parse().ok()?;
  let total: f64 = f.next().and_then(|s| s.parse().ok()).or_else(|| f.next()?.parse().ok())?;
  (total > 0.0).then(|| (done / total * 100.0).clamp(0.0, 100.0) as f32)
}

// SECTION: engine impl, run and version and install or update
fn net<E: ToString>(e: E) -> MediaError {
  MediaError::Network(e.to_string())
}

#[async_trait]
impl YtDlpEngine for DesktopEngine {
  async fn run(&self, args: &YtDlpArgs, progress: Option<ProgressSink>) -> Result<String> {
    match progress {
      Some(p) => self.exec(args, Some(&p)).await,
      None => tokio::time::timeout(QUERY_TIMEOUT, self.exec(args, None))
        .await
        .map_err(|_| MediaError::Engine("yt-dlp timed out".into()))?,
    }
  }

  async fn version(&self) -> Result<Option<String>> {
    if !self.bin().exists() {
      return Ok(None);
    }
    // an empty target is left out of the command line, `--version` takes none
    let out = self.run(&YtDlpArgs::new("").flag("--version"), None).await?;
    Ok(Some(out.trim().to_owned()))
  }

  async fn update(&self) -> Result<String> {
    log::info!("installing yt-dlp ({ASSET}) into {}", self.dir.display());
    let client = reqwest::Client::builder().user_agent("yamp").build().map_err(net)?;
    let get = |url: String| {
      let client = client.clone();
      async move { client.get(url).send().await?.error_for_status() }
    };

    let sums = get(format!("{BASE}/SHA2-256SUMS")).await.map_err(net)?.text().await.map_err(net)?;
    let want = sums
      .lines()
      .find_map(|l| {
        let mut p = l.split_whitespace();
        let hash = p.next()?;
        let name = p.next()?.trim_start_matches('*');
        (name == ASSET).then(|| hash.to_lowercase())
      })
      .ok_or_else(|| MediaError::Engine("no checksum published for this platform".into()))?;

    let bytes = get(format!("{BASE}/{ASSET}")).await.map_err(net)?.bytes().await.map_err(net)?;
    if format!("{:x}", Sha256::digest(&bytes)) != want {
      log::error!("yt-dlp download failed its checksum");
      return Err(MediaError::Engine("checksum mismatch, download discarded".into()));
    }

    tokio::fs::create_dir_all(&self.dir).await?;
    let (bin, new, bak) = (self.bin(), self.bin().with_extension("new"), self.bin().with_extension("bak"));
    tokio::fs::write(&new, &bytes).await?;
    #[cfg(unix)]
    {
      use std::os::unix::fs::PermissionsExt;
      tokio::fs::set_permissions(&new, std::fs::Permissions::from_mode(0o755)).await?;
    }
    if bin.exists() {
      // the previous build is kept so a broken release can be rolled back
      let _ = tokio::fs::rename(&bin, &bak).await;
    }
    tokio::fs::rename(&new, &bin).await?;

    match self.version().await {
      Ok(Some(v)) => Ok(v),
      _ => {
        log::warn!("new yt-dlp build did not start, restoring the previous one");
        let _ = tokio::fs::rename(&bak, &bin).await;
        Err(MediaError::Engine("new build failed to start, rolled back".into()))
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn progress_uses_exact_total_before_the_estimate() {
    assert_eq!(parse_progress("yamp-progress 50 200 NA"), Some(25.0));
    assert_eq!(parse_progress("yamp-progress 50 NA 100"), Some(50.0));
  }

  #[test]
  fn progress_ignores_unknown_and_foreign_lines() {
    assert_eq!(parse_progress("yamp-progress 50 NA NA"), None);
    assert_eq!(parse_progress("[download] 12.0% of 3MiB"), None);
  }

  /// Runs the real process path against a stand-in script, since yt-dlp itself needs the network.
  #[cfg(unix)]
  async fn engine_with_script(body: &str) -> (DesktopEngine, PathBuf) {
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!("yamp-engine-test-{}-{}", std::process::id(), body.len()));
    std::fs::create_dir_all(&dir).unwrap();
    let bin = dir.join(ASSET);
    std::fs::write(&bin, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    (DesktopEngine::new(&dir), dir)
  }

  #[cfg(unix)]
  #[tokio::test]
  async fn streams_progress_and_keeps_only_real_output() {
    use std::sync::{Arc, Mutex};
    let (engine, dir) = engine_with_script(
      "echo 'yamp-progress 50 100 NA'\necho '{\"id\":\"x\"}'\necho 'yamp-progress 100 100 NA'",
    )
    .await;
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink: ProgressSink = {
      let seen = seen.clone();
      Arc::new(move |p| seen.lock().unwrap().push(p))
    };
    let out = engine.run(&YtDlpArgs::new("t"), Some(sink)).await.unwrap();
    assert_eq!(out.trim(), r#"{"id":"x"}"#);
    assert_eq!(*seen.lock().unwrap(), vec![50.0, 100.0]);
    let _ = std::fs::remove_dir_all(dir);
  }

  #[cfg(unix)]
  #[tokio::test]
  async fn failure_reports_the_last_error_line() {
    let (engine, dir) = engine_with_script("echo 'ERROR: [youtube] x: Video unavailable' >&2\nexit 1").await;
    let err = engine.run(&YtDlpArgs::new("t"), None).await.unwrap_err();
    assert_eq!(err.to_string(), "[youtube] x: Video unavailable");
    let _ = std::fs::remove_dir_all(dir);
  }

  #[tokio::test]
  async fn missing_binary_is_not_installed() {
    let engine = DesktopEngine::new(std::env::temp_dir().join("yamp-engine-absent"));
    assert!(matches!(engine.run(&YtDlpArgs::new("t"), None).await, Err(MediaError::NotInstalled)));
    assert_eq!(engine.version().await.unwrap(), None);
  }
}
