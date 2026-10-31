import { useCallback, useEffect, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import {
  Alert,
  Box,
  Button,
  Paper,
  Stack,
  TextField,
  ToggleButton,
  ToggleButtonGroup,
  Typography,
} from "@mui/material";
import { COMMANDS } from "../lib/commands";
import { logger } from "../lib/log";
import type { AudioQuality, DownloadJob, StreamInfo, Track } from "../lib/types";
import EngineBar from "./media/EngineBar";
import { duration } from "./media/format";
import Player, { type Now } from "./media/Player";
import QueuePanel from "./media/QueuePanel";
import TrackList from "./media/TrackList";
import PlayQueuePanel from "./media/PlayQueuePanel";
import { useDownloadJobs } from "./media/useDownloadJobs";
import { useMediaSession } from "./media/useMediaSession";
import { MAX_QUEUED, useStreamQueue } from "./media/useStreamQueue";

const log = logger("media-demo");

type Meta = Pick<Track, "title" | "artist" | "thumbnail">;

const META_KEY = "media-demo.meta";

/** Job snapshots only carry an id, so track details survive a reload through local storage. */
const loadMeta = (): Record<string, Meta> => {
  try {
    return JSON.parse(localStorage.getItem(META_KEY) ?? "{}");
  } catch {
    return {};
  }
};

const saveMeta = (meta: Record<string, Meta>) => {
  try {
    localStorage.setItem(META_KEY, JSON.stringify(meta));
  } catch {
    // storage is only a convenience for labels
  }
};

/** The asset protocol guesses the type from the file contents, so the blob gets an explicit one. */
const MIME: Record<string, string> = { m4a: "audio/mp4", mp4: "audio/mp4", webm: "audio/webm", opus: "audio/ogg" };
const mimeOf = (path: string) => MIME[path.split(".").pop() ?? ""] ?? "audio/mp4";

/** What the list shows. Search, playlist and related results all end up here. */
type Listing = { title: string; tracks: Track[] };

/**
 * Stub that exercises every backend capability: provider queries, streaming, the in-memory play
 * queue, the on-disk download queue and playback of downloaded files. It is meant to be read and
 * thrown away, not extended.
 */
export default function MediaDemo() {
  const [query, setQuery] = useState("");
  const [playlistId, setPlaylistId] = useState("");
  const [quality, setQuality] = useState<AudioQuality>("high");
  const [listing, setListing] = useState<Listing>({ title: "Results", tracks: [] });
  const [detail, setDetail] = useState<Track | null>(null);
  const [now, setNow] = useState<Now | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { jobs, clearFinished } = useDownloadJobs();
  const playQueue = useStreamQueue(quality);

  // jobs only carry an id, the details are remembered here for display and for the system player
  const meta = useRef<Record<string, Meta>>(loadMeta());
  const titleOf = (id: string) => meta.current[id]?.title ?? id;
  const remember = (tracks: Track[]) => {
    for (const t of tracks) meta.current[t.id] = { title: t.title, artist: t.artist, thumbnail: t.thumbnail };
    saveMeta(meta.current);
  };

  const fail = useCallback((message: string) => {
    log.error(message);
    setError(message);
  }, []);

  const run = (what: string, task: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    return task()
      .catch((e) => fail(`${what}: ${e}`))
      .finally(() => setBusy(false));
  };

  // SECTION: provider queries

  const search = () =>
    run("search", async () => {
      const tracks = await COMMANDS.media.search({ query: query.trim() });
      setListing({ title: `Results for "${query.trim()}"`, tracks });
    });

  const loadPlaylist = () =>
    run("playlist", async () => {
      const p = await COMMANDS.media.playlist({ id: playlistId.trim() });
      remember(p.tracks);
      setListing({ title: `Playlist: ${p.title}`, tracks: p.tracks });
    });

  const showRelated = (t: Track) =>
    run("related", async () => {
      const tracks = await COMMANDS.media.related({ id: t.id });
      setListing({ title: `After "${t.title}"`, tracks });
    });

  const showInfo = (t: Track) =>
    run("track info", async () => setDetail(await COMMANDS.media.track({ id: t.id })));

  // SECTION: playback

  const startStream = (t: Track, info: StreamInfo) => {
    const kbps = info.bitrateKbps ? `${Math.round(info.bitrateKbps)} kbps` : "unknown bitrate";
    const until = info.expiresAt ? `expires ${new Date(info.expiresAt * 1000).toLocaleTimeString()}` : "no expiry";
    setNow({
      title: t.title,
      artist: t.artist,
      artwork: t.thumbnail ?? undefined,
      src: info.url,
      origin: "stream",
      detail: `${info.mime} · ${kbps} · ${until}`,
    });
  };

  // the URL expires, so a direct play resolves it right before playing and never stores it
  const playStream = (t: Track) =>
    run("stream", async () => startStream(t, await COMMANDS.media.stream({ id: t.id, quality })));

  // SECTION: play queue

  const enqueue = (t: Track) => {
    const result = playQueue.add(t);
    if (result === "full") fail(`play queue is full (${MAX_QUEUED})`);
  };

  /** Starts the head of the queue, or a specific entry. Also runs when a track ends. */
  const playNext = (id?: string) =>
    run("play queue", async () => {
      const next = await playQueue.take(id);
      if (next) startStream(next.track, next.stream);
    });

  useMediaSession(now, () => playNext());

  // SECTION: disk playback

  // object URL of the file playing from disk, kept so it can be released when the next one starts
  const diskUrl = useRef<string | null>(null);
  const releaseDiskUrl = () => {
    if (diskUrl.current) URL.revokeObjectURL(diskUrl.current);
    diskUrl.current = null;
  };
  useEffect(() => releaseDiskUrl, []);

  const playFromDisk = (job: DownloadJob) => {
    if (!job.file) return;
    const { path } = job.file;
    return run("play from disk", async () => {
      // The asset protocol answers a range request with at most about 1 MB, which WebKitGTK does
      // not follow up reliably. Larger files then fail (code 4) or play garbled. A plain fetch
      // has no range and returns the whole file, so the blob is what gets played.
      const res = await fetch(convertFileSrc(path));
      if (!res.ok) throw new Error(`asset request returned ${res.status}`);
      const url = URL.createObjectURL(new Blob([await res.arrayBuffer()], { type: mimeOf(path) }));
      releaseDiskUrl();
      diskUrl.current = url;
      const m = meta.current[job.id];
      setNow({
        title: titleOf(job.id),
        artist: m?.artist,
        artwork: m?.thumbnail ?? undefined,
        src: url,
        origin: "disk",
        detail: path,
      });
    });
  };

  // SECTION: download queue

  const download = (t: Track) => {
    remember([t]);
    COMMANDS.queue.enqueue({ id: t.id, quality }).catch((e) => fail(`enqueue: ${e}`));
  };

  const cancel = (id: string) => COMMANDS.queue.cancel({ id }).catch((e) => fail(`cancel: ${e}`));

  return (
    <Stack spacing={2} sx={{ p: 2 }}>
      <EngineBar onError={fail} />
      {error && (
        <Alert severity="error" onClose={() => setError(null)}>
          {error}
        </Alert>
      )}

      <Box sx={{ display: "flex", flexWrap: "wrap", gap: 1 }}>
        <TextField
          size="small"
          placeholder="Search tracks"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && query.trim() && !busy && search()}
          sx={{ flex: 1, minWidth: 180 }}
        />
        <Button variant="contained" onClick={search} disabled={busy || !query.trim()}>
          Search
        </Button>
        <TextField
          size="small"
          placeholder="Playlist ID"
          value={playlistId}
          onChange={(e) => setPlaylistId(e.target.value)}
          sx={{ width: 180 }}
        />
        <Button onClick={loadPlaylist} disabled={busy || !playlistId.trim()}>
          Load
        </Button>
        <ToggleButtonGroup
          size="small"
          exclusive
          value={quality}
          onChange={(_, v: AudioQuality | null) => v && setQuality(v)}
        >
          <ToggleButton value="low">Low</ToggleButton>
          <ToggleButton value="medium">Medium</ToggleButton>
          <ToggleButton value="high">High</ToggleButton>
        </ToggleButtonGroup>
      </Box>

      <Player now={now} onError={fail} onEnded={() => playNext()} />

      <Box sx={{ display: "grid", gridTemplateColumns: { xs: "1fr", md: "3fr 2fr" }, gap: 2 }}>
        <Box>
          <Typography variant="subtitle2">{listing.title}</Typography>
          <TrackList
            tracks={listing.tracks}
            onPlay={playStream}
            onEnqueue={enqueue}
            onDownload={download}
            onInfo={showInfo}
            onRelated={showRelated}
          />
        </Box>
        <Stack spacing={2}>
          <PlayQueuePanel
            entries={playQueue.entries}
            onPlay={playNext}
            onRemove={playQueue.remove}
            onNext={() => playNext()}
            onClear={playQueue.clear}
          />
          <QueuePanel jobs={jobs} titleOf={titleOf} onCancel={cancel} onPlay={playFromDisk} onClear={clearFinished} />
          {detail && (
            <Paper variant="outlined" sx={{ p: 1 }}>
              <Typography variant="subtitle2">{detail.title}</Typography>
              <Typography variant="body2">{detail.artist}</Typography>
              <Typography variant="caption" color="text.secondary" component="div">
                {[detail.album, detail.releaseYear, duration(detail.durationSecs), detail.id]
                  .filter((v) => v != null && v !== "")
                  .join(" · ")}
              </Typography>
            </Paper>
          )}
        </Stack>
      </Box>
    </Stack>
  );
}
