import { useSyncExternalStore } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { COMMANDS } from "@/lib/commands";
import { logger } from "@/lib/log";
import { isLocalFile } from "@/lib/local";
import { getSetting, isBoolean, isNumber, setSetting } from "@/lib/settings";
import type { AudioQuality, Track } from "@/lib/types";
import { IS_MOBILE } from "@/util/platform";
import { recordPlayed } from "./history";
import { loadPlayback, savePlayback } from "./persist";

/**
 * Playback for the whole app. One audio element lives here and not in a component, so music keeps
 * playing while screens come and go. Components read it through `usePlayer` and `usePlayerTime`
 * and act through `player`.
 *
 * The queue is kept in play order. With shuffle on it is a shuffled copy and `original` holds the
 * order to go back to when shuffle is turned off.
 */
const log = logger("player");

export type Repeat = "off" | "all" | "one";
const REPEATS: Repeat[] = ["off", "all", "one"];

export type PlayerState = {
  queue: Track[];
  /** -1 when the queue is empty. */
  index: number;
  playing: boolean;
  /** A stream is being resolved, nothing is audible yet. */
  loading: boolean;
  volume: number;
  muted: boolean;
  shuffle: boolean;
  repeat: Repeat;
  error: string | null;
};

/** Kept apart from the state, it changes several times a second and only the seek bar needs it. */
export type PlayerTime = { position: number; duration: number };

export const QUALITIES: AudioQuality[] = ["low", "medium", "high"];
export const isQuality = (v: unknown): v is AudioQuality => QUALITIES.includes(v as AudioQuality);

/** A track is restarted instead of left when it already played this long. */
const RESTART_AFTER_SECS = 3;
/** The playback position is saved at most this often while playing. */
const SAVE_EVERY_MS = 15_000;
/** A stream URL stays usable for hours, a prefetched one is dropped after this to be safe. */
const PREFETCH_MAX_AGE_MS = 20 * 60_000;
/** How many related tracks autoplay appends at once. */
const AUTOPLAY_BATCH = 15;

const audio = new Audio();
audio.preload = "auto";

let current: PlayerState | undefined;
let time: PlayerTime = { position: 0, duration: 0 };
let original: Track[] = [];
/** Bumped by every load, so a stream that resolves after a newer request is dropped. */
let token = 0;
let extending = false;
/** Stream URLs resolved ahead of time. Resolving is the slow part of starting a track. */
const prefetched = new Map<string, { url: string; at: number }>();
/** Ids whose lookup is under way. The next track can change several times in a row, and a lookup is a whole yt-dlp run. */
const resolving = new Set<string>();
/** Fetching the audio itself ahead of time fills the HTTP cache the main element reads from. */
const warm = new Audio();
warm.preload = "auto";
warm.muted = true;
let lastSave = 0;
let saveTimer: ReturnType<typeof setTimeout> | undefined;
const listeners = new Set<() => void>();

// The saved settings are not loaded yet when this module is imported, so the first read builds the state.
function initialState(): PlayerState {
  const volume = getSetting("volume", 1, isNumber);
  const muted = getSetting("muted", false, isBoolean);
  audio.volume = Math.min(1, Math.max(0, volume));
  audio.muted = muted;
  const repeat = getSetting<Repeat>("repeat", "off", (v): v is Repeat => REPEATS.includes(v as Repeat));
  // the last session comes back paused, nothing is resolved until play is pressed
  const saved = loadPlayback();
  if (saved) {
    original = saved.original ?? saved.queue;
    time = { position: saved.position, duration: saved.queue[saved.index].durationSecs ?? 0 };
  }
  return {
    queue: saved?.queue ?? [],
    index: saved?.index ?? -1,
    playing: false,
    loading: false,
    volume,
    muted,
    shuffle: getSetting("shuffle", false, isBoolean),
    repeat,
    error: null,
  };
}

const read = () => (current ??= initialState());

const set = (patch: Partial<PlayerState>) => {
  current = { ...read(), ...patch };
  listeners.forEach((l) => l());
  if ("queue" in patch || "index" in patch) {
    // a load changes several things in a row, so only the last one is saved
    clearTimeout(saveTimer);
    saveTimer = setTimeout(save, 1000);
  }
};

function save() {
  const { queue, index, shuffle } = read();
  lastSave = Date.now();
  savePlayback(queue, original, shuffle, index, time.position);
}

const timeListeners = new Set<() => void>();
const setTime = (position: number, duration = time.duration) => {
  time = { position, duration };
  timeListeners.forEach((l) => l());
};

export const getTime = () => (read(), time);

/** The state without a hook, for code that reacts to events outside of rendering. */
export const getPlayer = () => read();

/** Called when the position is moved on purpose, which the steady time updates cannot tell. */
const seekListeners = new Set<() => void>();
export const onSeek = (cb: () => void) => (seekListeners.add(cb), () => void seekListeners.delete(cb));

const shuffled = <T>(list: T[]): T[] => {
  const out = [...list];
  for (let i = out.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [out[i], out[j]] = [out[j], out[i]];
  }
  return out;
};

// SECTION: loading

/** Resolves the stream right before playing, since the URL expires and is never stored. */
async function start(index: number, resumeAt = 0, retried = false): Promise<void> {
  const track = read().queue[index];
  if (!track) return;
  const mine = ++token;
  set({ index, loading: true, error: null });
  setTime(resumeAt, track.durationSecs ?? 0);
  try {
    const url = await streamUrl(track);
    if (mine !== token) return;
    audio.src = url;
    if (resumeAt > 0) audio.currentTime = resumeAt;
    // a failure after this point reaches the error event
    audio.onerror = () => recover(mine, index, retried);
    await audio.play();
  } catch (e) {
    // AbortError is expected when a newer load replaced this one mid-flight
    if (mine !== token || (e as DOMException)?.name === "AbortError") return;
    log.error(`couldn't play ${track.title}:`, e);
    set({ loading: false, playing: false, error: String(e) });
  }
}

/**
 * A file on this device wins over a stream. It plays without a network and its URL does not
 * expire. Desktop gets a URL of the file server, since not every desktop webview plays from the
 * asset protocol, a phone gets the asset protocol.
 */
async function localUrl(track: Track): Promise<string | null> {
  if (!IS_MOBILE) return COMMANDS.local.url({ trackId: track.id });
  const path = await COMMANDS.local.path({ trackId: track.id });
  return path && convertFileSrc(path);
}

async function streamUrl(track: Track): Promise<string> {
  const local = await localUrl(track);
  if (local) return local;
  // a track from a folder has no provider to stream from, so a vanished file ends here
  if (isLocalFile(track.id)) throw new Error("the file of this song is not available");
  const hit = prefetched.get(track.id);
  prefetched.delete(track.id);
  if (hit && Date.now() - hit.at < PREFETCH_MAX_AGE_MS) return hit.url;
  return (await COMMANDS.media.stream({ id: track.id, quality: getSetting("quality", "high", isQuality) })).url;
}

/**
 * Resolves and starts loading the track after the playing one, when the setting allows it. It is
 * called again whenever the track that comes next may have changed, since a track that was just
 * queued would otherwise be looked up only when the one before it ends.
 */
function prefetchNext() {
  if (!getSetting("prefetch", true, isBoolean)) return;
  const { queue, index, repeat } = read();
  if (index < 0) return;
  const next = queue[index + 1] ?? (repeat === "all" ? queue[0] : undefined);
  if (!next || next.id === queue[index]?.id || prefetched.has(next.id) || resolving.has(next.id)) return;
  resolving.add(next.id);
  // a file on the device needs no warming up and a stream would only waste bandwidth
  localUrl(next)
    .then((local) => {
      if (local) return;
      return COMMANDS.media.stream({ id: next.id, quality: getSetting("quality", "high", isQuality) }).then((info) => {
        prefetched.set(next.id, { url: info.url, at: Date.now() });
        // only the nearest track is worth the bandwidth, older entries are not coming up
        for (const id of [...prefetched.keys()].slice(0, -2)) prefetched.delete(id);
        warm.src = info.url;
      });
    })
    .catch((e) => log.warn(`couldn't prefetch ${next.title}:`, e))
    .finally(() => resolving.delete(next.id));
}

/** A stream URL that expired while paused fails on resume, so it is resolved once more at the same spot. */
function recover(mine: number, index: number, retried: boolean) {
  if (mine !== token) return;
  const code = audio.error?.code;
  if (!retried) {
    void start(index, audio.currentTime, true);
    return;
  }
  const title = read().queue[index]?.title ?? "track";
  log.error(`playback of ${title} failed: code=${code} "${audio.error?.message ?? ""}"`);
  set({ loading: false, playing: false, error: `playback failed (code ${code})` });
}

/** Appends related tracks of the last one. Returns whether anything new was added. */
async function extend(): Promise<boolean> {
  const { queue } = read();
  const last = queue[queue.length - 1];
  if (!last || extending) return false;
  extending = true;
  try {
    const related = await COMMANDS.media.related({ id: last.id, limit: AUTOPLAY_BATCH });
    const known = new Set(read().queue.map((t) => t.id));
    const fresh = related.filter((t) => !known.has(t.id));
    if (fresh.length === 0) return false;
    original = [...original, ...fresh];
    set({ queue: [...read().queue, ...fresh] });
    return true;
  } catch (e) {
    log.warn("autoplay couldn't find related tracks:", e);
    return false;
  } finally {
    extending = false;
  }
}

// SECTION: audio events

audio.addEventListener("playing", () => {
  set({ playing: true, loading: false });
  const { queue, index } = read();
  if (queue[index]) recordPlayed(queue[index]);
  prefetchNext();
});
audio.addEventListener("pause", () => {
  set({ playing: false });
  save();
});
audio.addEventListener("timeupdate", () => {
  setTime(audio.currentTime);
  if (Date.now() - lastSave > SAVE_EVERY_MS) save();
});
// a closed or backgrounded app may be killed without a pause event
document.addEventListener("visibilitychange", () => document.hidden && current && save());
window.addEventListener("pagehide", () => current && save());
audio.addEventListener("durationchange", () => {
  if (Number.isFinite(audio.duration)) setTime(audio.currentTime, audio.duration);
});
audio.addEventListener("ended", () => {
  if (read().repeat === "one") {
    audio.currentTime = 0;
    void audio.play();
  } else {
    void advance(true);
  }
});

async function advance(auto: boolean) {
  const { queue, index, repeat } = read();
  const autoplay = getSetting("autoplay", true, isBoolean);
  if (index < queue.length - 1) return start(index + 1);
  if (repeat === "all" && queue.length > 0) return start(0);
  if (autoplay && queue.length > 0 && (await extend())) return start(index + 1);
  // the end of the queue, a finished track is left as it is
  if (auto) set({ playing: false });
}

// SECTION: actions

export const player = {
  /** Replaces the queue and starts at `startIndex`. */
  play(tracks: Track[], startIndex = 0) {
    if (tracks.length === 0) return;
    const first = tracks[startIndex] ?? tracks[0];
    original = tracks;
    if (read().shuffle) {
      const rest = tracks.filter((t) => t !== first);
      set({ queue: [first, ...shuffled(rest)] });
      void start(0);
    } else {
      set({ queue: tracks });
      void start(tracks.indexOf(first));
    }
  },

  /** Plays an entry that is already in the queue. */
  jumpTo(index: number) {
    void start(index);
  },

  togglePlay() {
    const { index, error } = read();
    if (index < 0) return;
    // after a failure the element holds a dead source, so the track is loaded again
    if (error || !audio.src) void start(index, time.position);
    else if (audio.paused) void audio.play();
    else audio.pause();
  },

  /** Leaves the position as it is, unlike `togglePlay` this never starts anything. */
  pause() {
    audio.pause();
  },

  next() {
    void advance(false);
  },

  prev() {
    const { index, queue, repeat } = read();
    if (index < 0) return;
    if (audio.currentTime > RESTART_AFTER_SECS || (index === 0 && repeat !== "all")) {
      player.seek(0);
    } else {
      void start(index > 0 ? index - 1 : queue.length - 1);
    }
  },

  seek(secs: number) {
    audio.currentTime = secs;
    setTime(secs);
    seekListeners.forEach((l) => l());
  },

  /** `next` puts the tracks right after the playing one, `end` at the back. */
  enqueue(tracks: Track[], where: "next" | "end") {
    const { queue, index } = read();
    if (queue.length === 0) return player.play(tracks);
    const at = where === "next" ? index + 1 : queue.length;
    set({ queue: [...queue.slice(0, at), ...tracks, ...queue.slice(at)] });
    const playing = original.indexOf(queue[index]);
    const originalAt = where === "next" && playing >= 0 ? playing + 1 : original.length;
    original = [...original.slice(0, originalAt), ...tracks, ...original.slice(originalAt)];
    prefetchNext();
  },

  removeAt(i: number) {
    const { queue, index } = read();
    const [gone] = queue.slice(i, i + 1);
    if (!gone) return;
    const next = queue.filter((_, n) => n !== i);
    const inOriginal = original.indexOf(gone);
    if (inOriginal >= 0) original = original.filter((_, n) => n !== inOriginal);
    if (i !== index) {
      set({ queue: next, index: i < index ? index - 1 : index });
      prefetchNext();
    } else if (next.length === 0) {
      set({ queue: next });
      player.clear();
    } else {
      set({ queue: next });
      void start(Math.min(i, next.length - 1));
    }
  },

  clear() {
    token++;
    audio.pause();
    audio.removeAttribute("src");
    audio.load();
    original = [];
    set({ queue: [], index: -1, playing: false, loading: false, error: null });
    setTime(0, 0);
  },

  setVolume(volume: number, save = true) {
    const v = Math.min(1, Math.max(0, volume));
    audio.volume = v;
    // moving the slider while muted is a request to hear it again
    const muted = v > 0 ? false : read().muted;
    audio.muted = muted;
    set({ volume: v, muted });
    if (save) {
      setSetting("volume", v);
      setSetting("muted", muted);
    }
  },

  toggleMute() {
    const muted = !read().muted;
    audio.muted = muted;
    set({ muted });
    setSetting("muted", muted);
  },

  toggleShuffle() {
    const { shuffle, queue, index } = read();
    const on = !shuffle;
    setSetting("shuffle", on);
    const playing = queue[index];
    if (!playing) return set({ shuffle: on });
    if (on) {
      original = queue;
      const rest = queue.filter((_, n) => n !== index);
      set({ shuffle: on, queue: [playing, ...shuffled(rest)], index: 0 });
    } else {
      set({ shuffle: on, queue: original, index: Math.max(0, original.indexOf(playing)) });
    }
    prefetchNext();
  },

  cycleRepeat() {
    const repeat = REPEATS[(REPEATS.indexOf(read().repeat) + 1) % REPEATS.length];
    setSetting("repeat", repeat);
    set({ repeat });
    // repeating all makes the first track the one after the last
    prefetchNext();
  },
};

// SECTION: hooks

const subscribe = (cb: () => void) => (listeners.add(cb), () => void listeners.delete(cb));
const subscribeTime = (cb: () => void) => (timeListeners.add(cb), () => void timeListeners.delete(cb));

export const usePlayer = () => useSyncExternalStore(subscribe, read);
export const usePlayerTime = () => useSyncExternalStore(subscribeTime, getTime);

/** The playing track, or undefined when the queue is empty. */
export const currentTrack = (s: PlayerState): Track | undefined => s.queue[s.index];
