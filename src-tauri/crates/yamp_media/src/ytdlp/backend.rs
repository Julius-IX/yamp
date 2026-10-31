use super::{
  engine::{YtDlpArgs, YtDlpEngine},
  parse,
};
use crate::{
  backend::{Downloader, MediaSource, ProgressSink, Updater},
  error::{MediaError, Result},
  model::{AudioQuality, DownloadedFile, EngineStatus, Playlist, StreamInfo, Track},
};
use async_trait::async_trait;
use std::{
  path::{Path, PathBuf},
  time::SystemTime,
};

const MAX_LIMIT: usize = 50;

pub struct YtDlpBackend<E> {
  engine: E,
}

impl<E: YtDlpEngine> YtDlpBackend<E> {
  pub fn new(engine: E) -> Self {
    Self { engine }
  }
}

// SECTION: helpers
fn urlencode(s: &str) -> String {
  s.bytes()
    .map(|b| match b {
      b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
      _ => format!("%{b:02X}"),
    })
    .collect()
}

/// Ids end up inside URLs, so only the characters YouTube itself uses are accepted.
fn check_id(id: &str) -> Result<()> {
  let ok = !id.is_empty()
    && id.len() <= 64
    && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
  ok.then_some(()).ok_or(MediaError::InvalidId)
}

fn watch_url(id: &str) -> String {
  format!("https://www.youtube.com/watch?v={id}")
}

fn selector(q: AudioQuality) -> &'static str {
  // m4a is preferred at every level because AAC plays in all target webviews, while opus in
  // webm does not play in WebKit based ones.
  match q {
    AudioQuality::High => "bestaudio[ext=m4a]/bestaudio",
    AudioQuality::Medium => "bestaudio[ext=m4a][abr<=130]/bestaudio[abr<=130]/bestaudio",
    AudioQuality::Low => "worstaudio[ext=m4a]/worstaudio",
  }
}

fn flat_listing(target: String, limit: usize) -> YtDlpArgs {
  YtDlpArgs::new(target)
    .flag("--flat-playlist")
    .flag("--dump-json")
    .flag("--no-warnings")
    .opt("--playlist-end", limit.to_string())
}

// SECTION: media source
#[async_trait]
impl<E: YtDlpEngine> MediaSource for YtDlpBackend<E> {
  async fn search(&self, query: &str, limit: usize) -> Result<Vec<Track>> {
    let q = query.trim();
    if q.is_empty() {
      return Ok(vec![]);
    }
    let limit = limit.clamp(1, MAX_LIMIT);

    // YouTube Music first, since it lists songs only
    let music = format!("https://music.youtube.com/search?q={}", urlencode(q));
    match self.engine.run(&flat_listing(music, limit), None).await {
      Ok(out) => {
        let tracks = parse::entries(&out, true);
        if !tracks.is_empty() {
          return Ok(tracks);
        }
        log::debug!("music search had no usable results, using plain youtube search");
      }
      Err(MediaError::NotInstalled) => return Err(MediaError::NotInstalled),
      Err(e) => log::warn!("music search failed, using plain youtube search: {e}"),
    }

    // plain YouTube search whenever music gave nothing usable
    let out = self.engine.run(&flat_listing(format!("ytsearch{limit}:{q}"), limit), None).await?;
    Ok(parse::entries(&out, true))
  }

  async fn track(&self, id: &str) -> Result<Track> {
    check_id(id)?;
    let args = YtDlpArgs::new(watch_url(id))
      .flag("--dump-json")
      .flag("--no-playlist")
      .flag("--no-warnings");
    parse::track(&self.engine.run(&args, None).await?)
  }

  async fn playlist(&self, id: &str) -> Result<Playlist> {
    check_id(id)?;
    let args = YtDlpArgs::new(format!("https://www.youtube.com/playlist?list={id}"))
      .flag("--flat-playlist")
      .flag("--dump-single-json")
      .flag("--no-warnings");
    parse::playlist(&self.engine.run(&args, None).await?)
  }

  async fn related(&self, id: &str, limit: usize) -> Result<Vec<Track>> {
    check_id(id)?;
    let limit = limit.clamp(1, MAX_LIMIT);
    // `RD<id>` is the auto generated mix of a video. It starts with the video itself, so one
    // extra entry is requested to still end up with `limit` after dropping it.
    let target = format!("https://www.youtube.com/watch?v={id}&list=RD{id}");
    let out = self.engine.run(&flat_listing(target, limit + 1), None).await?;
    let mut tracks = parse::entries(&out, true);
    tracks.retain(|t| t.id != id);
    tracks.truncate(limit);
    Ok(tracks)
  }

  async fn stream(&self, id: &str, quality: AudioQuality) -> Result<StreamInfo> {
    check_id(id)?;
    // `--dump-json` with a format selector prints the chosen format at the top level, which
    // gives the container and bitrate along with the URL.
    let args = YtDlpArgs::new(watch_url(id))
      .opt("-f", selector(quality))
      .flag("--dump-json")
      .flag("--no-playlist")
      .flag("--no-warnings");
    parse::stream(&self.engine.run(&args, None).await?)
  }
}

// SECTION: downloader
/// Newest finished file for `id`. Quality changes can leave several containers behind, and
/// yt-dlp's own `.part` and `.ytdl` leftovers must never be returned.
async fn find_download(dir: &Path, id: &str) -> Result<(PathBuf, u64)> {
  let prefix = format!("{id}.");
  let mut best: Option<(SystemTime, PathBuf, u64)> = None;
  let mut rd = tokio::fs::read_dir(dir).await?;
  while let Some(entry) = rd.next_entry().await? {
    let path = entry.path();
    let name = entry.file_name().to_string_lossy().into_owned();
    let unfinished = matches!(path.extension().and_then(|e| e.to_str()), Some("part" | "ytdl" | "temp"));
    if !name.starts_with(&prefix) || unfinished {
      continue;
    }
    let meta = entry.metadata().await?;
    let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    if best.as_ref().is_none_or(|(t, ..)| modified > *t) {
      best = Some((modified, path, meta.len()));
    }
  }
  best.map(|(_, p, n)| (p, n)).ok_or(MediaError::NotFound)
}

#[async_trait]
impl<E: YtDlpEngine> Downloader for YtDlpBackend<E> {
  async fn download(
    &self,
    id: &str,
    quality: AudioQuality,
    dir: &Path,
    progress: Option<ProgressSink>,
  ) -> Result<DownloadedFile> {
    check_id(id)?;
    tokio::fs::create_dir_all(dir).await?;
    let template = dir
      .join("%(id)s.%(ext)s")
      .to_str()
      .ok_or_else(|| MediaError::Io("download path is not valid UTF-8".into()))?
      .to_owned();
    // No ffmpeg step on purpose. The raw container is kept so nothing but yt-dlp is needed
    // on any platform.
    let args = YtDlpArgs::new(watch_url(id))
      .opt("-f", selector(quality))
      .opt("-o", template)
      .flag("--no-playlist")
      .flag("--no-warnings")
      .flag("--no-mtime");
    self.engine.run(&args, progress).await?;

    let (path, size_bytes) = find_download(dir, id).await?;
    Ok(DownloadedFile { id: id.to_owned(), path, size_bytes })
  }
}

// SECTION: updater
#[async_trait]
impl<E: YtDlpEngine> Updater for YtDlpBackend<E> {
  async fn status(&self) -> Result<EngineStatus> {
    let version = self.engine.version().await?;
    Ok(EngineStatus { installed: version.is_some(), version })
  }

  async fn update(&self) -> Result<EngineStatus> {
    let version = self.engine.update().await?;
    log::info!("yt-dlp is now at {version}");
    Ok(EngineStatus { installed: true, version: Some(version) })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::sync::Mutex;

  /// Replays canned outputs in call order and records the arguments it was given.
  struct Mock {
    outputs: Mutex<Vec<Result<String>>>,
    calls: Mutex<Vec<YtDlpArgs>>,
  }

  impl Mock {
    fn new(outputs: Vec<Result<String>>) -> Self {
      Self { outputs: Mutex::new(outputs.into_iter().rev().collect()), calls: Default::default() }
    }
  }

  #[async_trait]
  impl YtDlpEngine for Mock {
    async fn run(&self, args: &YtDlpArgs, _: Option<ProgressSink>) -> Result<String> {
      self.calls.lock().unwrap().push(args.clone());
      self.outputs.lock().unwrap().pop().expect("unexpected extra call")
    }
    async fn version(&self) -> Result<Option<String>> {
      Ok(None)
    }
    async fn update(&self) -> Result<String> {
      Ok("1".into())
    }
  }

  const SONG: &str = r#"{"id":"aaaaaaaaaaa","title":"Song","duration":200.0}"#;

  #[tokio::test]
  async fn search_falls_back_to_plain_youtube() {
    let b = YtDlpBackend::new(Mock::new(vec![Err(MediaError::Engine("boom".into())), Ok(SONG.into())]));
    let tracks = b.search("a b", 10).await.unwrap();
    assert_eq!(tracks.len(), 1);
    let calls = b.engine.calls.lock().unwrap();
    assert_eq!(calls[0].target, "https://music.youtube.com/search?q=a%20b");
    assert_eq!(calls[1].target, "ytsearch10:a b");
  }

  #[tokio::test]
  async fn search_does_not_hide_a_missing_engine() {
    let b = YtDlpBackend::new(Mock::new(vec![Err(MediaError::NotInstalled)]));
    assert!(matches!(b.search("x", 5).await, Err(MediaError::NotInstalled)));
  }

  #[tokio::test]
  async fn related_drops_the_seed() {
    let seed = r#"{"id":"seed","title":"Seed","duration":100.0}"#;
    let b = YtDlpBackend::new(Mock::new(vec![Ok(format!("{seed}\n{SONG}\n"))]));
    let tracks = b.related("seed", 5).await.unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].id, "aaaaaaaaaaa");
  }

  #[tokio::test]
  async fn bad_ids_never_reach_the_engine() {
    let b = YtDlpBackend::new(Mock::new(vec![]));
    assert!(matches!(b.track("a b").await, Err(MediaError::InvalidId)));
    assert!(matches!(b.stream("../x", AudioQuality::High).await, Err(MediaError::InvalidId)));
  }

  #[tokio::test]
  async fn download_returns_the_finished_file_only() {
    let dir = std::env::temp_dir().join(format!("yamp-media-test-{}", std::process::id()));
    tokio::fs::create_dir_all(&dir).await.unwrap();
    tokio::fs::write(dir.join("abc.m4a.part"), b"x").await.unwrap();
    tokio::fs::write(dir.join("abc.m4a"), b"12345").await.unwrap();
    let b = YtDlpBackend::new(Mock::new(vec![Ok(String::new())]));
    let f = b.download("abc", AudioQuality::High, &dir, None).await.unwrap();
    assert_eq!((f.path.file_name().unwrap().to_str().unwrap(), f.size_bytes), ("abc.m4a", 5));
    let _ = tokio::fs::remove_dir_all(&dir).await;
  }
}
