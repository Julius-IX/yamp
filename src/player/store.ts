import { useSyncExternalStore } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { COMMANDS } from "@/lib/commands";
import { logger } from "@/lib/log";
import { isLocalFile } from "@/lib/local";
import { getSetting, isBoolean, isNumber, onSettingChange, rawSetting, setSetting } from "@/lib/settings";
import type { AudioQuality, Track } from "@/lib/types";
import { resolveStream } from "@/radio/api";
import { isRadio, stationId } from "@/radio/station";
import { IS_MOBILE } from "@/util/platform";
import { flushStats, trackListen } from "@/stats/record";
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
/** How many of the upcoming tracks are resolved ahead of time. `all` is the rest of the queue. */
export const PREFETCH_AMOUNTS = ["off", "1", "3", "5", "10", "all"] as const;
export type PrefetchAmount = (typeof PREFETCH_AMOUNTS)[number];
export const isPrefetchAmount = (v: unknown): v is PrefetchAmount => PREFETCH_AMOUNTS.includes(v as PrefetchAmount);

/** Before the amount there was a switch, and a device that turned it off stays off. */
export const defaultPrefetchAmount = (): PrefetchAmount => (rawSetting("prefetch") === false ? "off" : "1");

const prefetchLimit = () => {
  const amount = getSetting("prefetchAmount", defaultPrefetchAmount(), isPrefetchAmount);
  return amount === "off" ? 0 : amount === "all" ? Infinity : Number(amount);
};

export const isQuality = (v: unknown): v is AudioQuality => QUALITIES.includes(v as AudioQuality);

/** A track is restarted instead of left when it already played this long. */
const RESTART_AFTER_SECS = 3;
/** The playback position is saved at most this often while playing. */
const SAVE_EVERY_MS = 15_000;
/**
 * A stream URL stays usable for hours, a prefetched one is dropped after this to be safe. It is
 * also how far ahead is looked, a track that would start later than this from now is left for later.
 */
const PREFETCH_MAX_AGE_MS = 60 * 60_000;
/** How long a failed lookup ahead of time is left alone before it is tried again. */
const PREFETCH_RETRY_MS = 15_000;
/** How many related tracks autoplay appends at once. */
const AUTOPLAY_BATCH = 15;
/** The next track is loaded into a second element once this little of the playing one is left. */
const WARM_AHEAD_SECS = 20;

let audio = new Audio();
// "auto" lets WebKitGTK buffer ahead in its GStreamer pipeline, which is held in the web process.
// Next-track warm-up is done by resolving URLs (see prefetchNext), not by the element.
audio.preload = "metadata";

let current: PlayerState | undefined;
let time: PlayerTime = { position: 0, duration: 0 };
let original: Track[] = [];
/** Bumped by every load, so a stream that resolves after a newer request is dropped. */
let token = 0;
/** Bumped by every pick of something to play. See `beginPick`. */
let picks = 0;
/** The last track autoplay already looked ahead from. See `extendAhead`. */
let extendedFor = "";
/** The next track, already loading in an element of its own so the switch needs no network. */
let standby: { id: string; el: HTMLAudioElement } | null = null;
let warming = "";
const warmFailed = new Map<string, number>();
/** Stream URLs resolved ahead of time. Resolving is the slow part of starting a track. */
const prefetched = new Map<string, { url: string; at: number }>();
/** Ids whose lookup is under way. The next track can change several times in a row, and a lookup is a whole yt-dlp run. */
const resolving = new Set<string>();
/** Ids whose last lookup ahead of time failed, and when. A phone loses its network often, so a lookup is tried again after a pause and not only when the playing track changes. */
const prefetchFailed = new Map<string, number>();
/** Ids that needed no lookup because their file is on this device, and when that was seen. A download can be removed, so it is looked at again after a pause. */
const prefetchSkipped = new Map<string, number>();
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

/**
 * A new order for another pass through a shuffled queue. Replaying the order of the last pass
 * would make shuffle with repeat all a fixed loop. The track that just ended is kept off the
 * front so it does not play twice in a row.
 */
const nextPass = (list: Track[], ended: Track | undefined): Track[] => {
  const out = shuffled(list);
  if (out.length > 1 && out[0].id === ended?.id) {
    const j = 1 + Math.floor(Math.random() * (out.length - 1));
    [out[0], out[j]] = [out[j], out[0]];
  }
  return out;
};

/**
 * For a list that has to be loaded before it can play. The list takes a number first and checks
 * it with `stillWanted` once it has loaded, so a slow one does not replace a song that was
 * picked in the meantime.
 */
export const beginPick = () => ++picks;
export const stillWanted = (pick: number) => pick === picks;

// SECTION: loading

/** Resolves the stream right before playing, since the URL expires and is never stored. */
async function start(index: number, resumeAt = 0, retried = false): Promise<void> {
  const track = read().queue[index];
  if (!track) return;
  const mine = ++token;
  set({ index, loading: true, error: null });
  setTime(resumeAt, track.durationSecs ?? 0);
  try {
    // a track that was loaded ahead of time starts at once, see `warmNext`
    let warm: HTMLAudioElement | null = null;
    if (resumeAt === 0) warm = takeStandby(track.id);
    else dropStandby();
    const url = warm ? "" : await streamUrl(track);
    if (mine !== token) {
      if (warm) release(warm);
      return;
    }
    if (warm) {
      const old = audio;
      // events of the old element are ignored from here on, see `wire`
      audio = warm;
      old.pause();
      audio.volume = read().volume;
      audio.muted = read().muted;
      audio.preload = "metadata";
      // its duration was announced while it was still ignored
      if (Number.isFinite(audio.duration)) setTime(0, audio.duration);
      audio.onerror = () => recover(mine, index, retried);
      // the finished element is torn down once the next one is audible, not before
      await audio.play().finally(() => release(old));
    } else {
      // Tear the previous media pipeline down before building the next one. Swapping `src` alone
      // can leave the old GStreamer pipeline and its buffers alive in WebKitGTK.
      audio.removeAttribute("src");
      audio.load();
      audio.src = url;
      // a live stream has no position to return to
      if (resumeAt > 0 && !isRadio(track.id)) audio.currentTime = resumeAt;
      // a failure after this point reaches the error event
      audio.onerror = () => recover(mine, index, retried);
      await audio.play();
    }
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

async function streamUrl(track: Track, now = true): Promise<string> {
  if (isRadio(track.id)) return resolveStream(stationId(track.id));
  const local = await localUrl(track);
  if (local) return local;
  // a track from a folder has no provider to stream from, so a vanished file ends here
  if (isLocalFile(track.id)) throw new Error("the file of this song is not available");
  const hit = prefetched.get(track.id);
  prefetched.delete(track.id);
  if (hit && Date.now() - hit.at < PREFETCH_MAX_AGE_MS) return hit.url;
  // `now` lets the backend stop the lookup of a track that was picked before this one
  return (await COMMANDS.media.stream({ id: track.id, quality: getSetting("quality", "high", isQuality), now })).url;
}

/**
 * The tracks after the playing one that the setting asks to be resolved, nearest first. A track
 * that would start further ahead than the age of a prefetched URL is left out, since its URL
 * would be dropped before it was used. That also keeps the entire queue from being looked up
 * at once.
 */
function prefetchTargets(): Track[] {
  const limit = prefetchLimit();
  const { queue, index, repeat } = read();
  if (limit === 0 || index < 0) return [];
  const targets: Track[] = [];
  let waitSecs = Math.max(0, (time.duration || queue[index]?.durationSecs || 0) - time.position);
  // `step` stops short of the playing track, which comes round again when the queue repeats
  for (let step = 1; step < queue.length && targets.length < limit; step++) {
    const at = index + step;
    const track = queue[repeat === "all" ? at % queue.length : at];
    if (!track || waitSecs * 1000 > PREFETCH_MAX_AGE_MS) break;
    targets.push(track);
    waitSecs += track.durationSecs ?? 0;
  }
  return targets;
}

const recently = (seen: Map<string, number>, id: string) => Date.now() - (seen.get(id) ?? 0) < PREFETCH_RETRY_MS;

/**
 * Resolves the stream of the upcoming tracks, as many as the setting allows. It is called again
 * whenever the tracks that come next may have changed, since a track that was just queued would
 * otherwise be looked up only when the one before it ends. A lookup is a whole yt-dlp run, so
 * they go one after the other, each one starting the next when it is done. Only the nearest track
 * does not wait for a lookup that is under way, it may have changed while that one ran.
 */
function prefetchNext() {
  const targets = prefetchTargets();
  const { queue, index } = read();
  const fresh = (id: string) => {
    const hit = prefetched.get(id);
    return !!hit && Date.now() - hit.at < PREFETCH_MAX_AGE_MS;
  };
  const next = targets.find(
    (t) =>
      // a station is live, so there is nothing to warm up and looking it up early would count a listener
      !isRadio(t.id) && t.id !== queue[index]?.id && !fresh(t.id) && !resolving.has(t.id) &&
      !recently(prefetchFailed, t.id) && !recently(prefetchSkipped, t.id),
  );
  if (!next || (resolving.size > 0 && next !== targets[0])) return;
  resolving.add(next.id);
  // a file on the device needs no warming up and a stream would only waste bandwidth
  localUrl(next)
    .then((local) => {
      if (local) return void prefetchSkipped.set(next.id, Date.now());
      return COMMANDS.media.stream({ id: next.id, quality: getSetting("quality", "high", isQuality) }).then((info) => {
        prefetched.set(next.id, { url: info.url, at: Date.now() });
        prefetchFailed.delete(next.id);
        // the oldest are the ones least likely to still be coming up
        const keep = Math.max(2, Math.min(prefetchLimit(), 50)) + 1;
        for (const id of [...prefetched.keys()].slice(0, -keep)) prefetched.delete(id);
      });
    })
    .catch((e) => {
      prefetchFailed.set(next.id, Date.now());
      log.warn(`couldn't prefetch ${next.title}:`, e);
    })
    .finally(() => {
      resolving.delete(next.id);
      prefetchNext();
    });
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

/** The lookup that is under way, shared so a track that ends meanwhile waits for it. */
let extension: Promise<boolean> | null = null;

/** Appends related tracks of the last one. Returns whether anything new was added. */
function extend(): Promise<boolean> {
  return (extension ??= lookUpRelated().finally(() => (extension = null)));
}

async function lookUpRelated(): Promise<boolean> {
  const { queue } = read();
  const last = queue[queue.length - 1];
  // related tracks are looked up at the music provider, which does not know a station
  if (!last || isRadio(last.id)) return false;
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
  }
}

/**
 * Looks up the autoplay tracks while the last track of the queue is still playing. They are
 * then queued like any other, so the stream of the first one is resolved and loaded ahead and the
 * end of a list has no pause. Each last track is tried once, a lookup that found nothing is not
 * repeated every second. Ending still tries again, see `advance`.
 */
function extendAhead() {
  const { queue, index, repeat } = read();
  const last = queue[queue.length - 1];
  if (!last || index !== queue.length - 1 || repeat !== "off" || extendedFor === last.id) return;
  if (!getSetting("autoplay", true, isBoolean)) return;
  extendedFor = last.id;
  void extend().then((added) => added && prefetchNext());
}

/** Frees an element's media pipeline. Pausing alone keeps the buffers of the source. */
function release(el: HTMLAudioElement) {
  el.pause();
  el.removeAttribute("src");
  el.load();
}

function dropStandby() {
  if (standby) release(standby.el);
  standby = null;
}

/** The loaded-ahead element when it holds this track and is healthy, otherwise nothing and it is dropped. */
function takeStandby(id: string): HTMLAudioElement | null {
  const held = standby;
  standby = null;
  if (!held) return null;
  if (held.id !== id || held.el.error) {
    release(held.el);
    return null;
  }
  return held.el;
}

/** Loads the next track into a second element, so that the switch does not wait for a connection. */
async function warm(next: Track) {
  warming = next.id;
  try {
    const url = await streamUrl(next, false);
    // the queue may have moved on while the address was being resolved
    const { queue, index } = read();
    if ((queue[index + 1] ?? queue[0])?.id !== next.id) return;
    dropStandby();
    const el = new Audio();
    el.preload = "auto";
    wire(el);
    el.src = url;
    el.load();
    standby = { id: next.id, el };
  } catch (e) {
    warmFailed.set(next.id, Date.now());
    log.warn(`couldn't load ${next.title} ahead of time:`, e);
  } finally {
    warming = "";
  }
}

/** Called by every time update. Cheap when there is nothing to do. */
function warmNext() {
  if (prefetchLimit() === 0) return;
  if (!Number.isFinite(audio.duration) || audio.duration - audio.currentTime > WARM_AHEAD_SECS) return;
  const { queue, index, repeat } = read();
  const now = queue[index];
  if (!now || isRadio(now.id) || repeat === "one") return;
  const next = queue[index + 1] ?? (repeat === "all" ? queue[0] : undefined);
  if (!next || isRadio(next.id) || next.id === now.id) return;
  if (standby?.id === next.id || warming === next.id) return;
  if (Date.now() - (warmFailed.get(next.id) ?? 0) < PREFETCH_RETRY_MS) return;
  void warm(next);
}

// SECTION: audio events

/** Only the audible element drives the state, a standby or a released one is ignored. */
function wire(el: HTMLAudioElement) {
  el.addEventListener("playing", () => {
    if (el !== audio) return;
    set({ playing: true, loading: false });
    const { queue, index } = read();
    // the history feeds the recommendations, which are made of songs
    if (queue[index] && !isRadio(queue[index].id)) recordPlayed(queue[index]);
    prefetchNext();
  });
  el.addEventListener("pause", () => {
    if (el !== audio) return;
    set({ playing: false });
    save();
    void flushStats();
  });
  el.addEventListener("timeupdate", () => {
    if (el !== audio) return;
    setTime(el.currentTime);
    const { queue, index } = read();
    if (queue[index]) trackListen(queue[index], el.currentTime);
    extendAhead();
    // cheap when there is nothing to do, and it is what retries a lookup that failed
    prefetchNext();
    warmNext();
    if (Date.now() - lastSave > SAVE_EVERY_MS) save();
  });
  el.addEventListener("durationchange", () => {
    if (el !== audio) return;
    if (Number.isFinite(el.duration)) setTime(el.currentTime, el.duration);
  });
  el.addEventListener("ended", () => {
    if (el !== audio) return;
    const { queue, index } = read();
    // a live stream only ends when the connection drops, moving on would skip to another station
    if (queue[index] && isRadio(queue[index].id)) {
      set({ playing: false, error: "the stream ended" });
      return;
    }
    if (read().repeat === "one") {
      el.currentTime = 0;
      void el.play();
    } else {
      void advance(true);
    }
  });
}
wire(audio);
// a closed or backgrounded app may be killed without a pause event
document.addEventListener("visibilitychange", () => document.hidden && current && save());
window.addEventListener("pagehide", () => current && save());

async function advance(auto: boolean) {
  const { queue, index, repeat } = read();
  const autoplay = getSetting("autoplay", true, isBoolean);
  if (index < queue.length - 1) return start(index + 1);
  if (repeat === "all" && queue.length > 0) {
    if (read().shuffle) set({ queue: nextPass(queue, queue[index]) });
    return start(0);
  }
  if (autoplay && queue.length > 0 && (await extend())) return start(index + 1);
  // the end of the queue, a finished track is left as it is
  if (auto) set({ playing: false });
}

// SECTION: actions

export const player = {
  /** Replaces the queue and starts at `startIndex`. */
  play(tracks: Track[], startIndex = 0) {
    if (tracks.length === 0) return;
    picks++;
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
    picks++;
    void start(index);
  },

  togglePlay() {
    const { index, error } = read();
    if (index < 0) return;
    // after a failure the element holds a dead source, so the track is loaded again. A paused
    // station is loaded again as well, since the audio that was buffered is no longer live
    const live = isRadio(read().queue[index]?.id ?? "");
    if (error || !audio.src || (live && audio.paused)) void start(index, time.position);
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
    dropStandby();
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

// the amount can be changed while a queue plays, which should be felt without waiting for the next track
onSettingChange((key) => key === "prefetchAmount" && prefetchNext());

// SECTION: hooks

const subscribe = (cb: () => void) => (listeners.add(cb), () => void listeners.delete(cb));
const subscribeTime = (cb: () => void) => (timeListeners.add(cb), () => void timeListeners.delete(cb));

export const usePlayer = () => useSyncExternalStore(subscribe, read);
export const usePlayerTime = () => useSyncExternalStore(subscribeTime, getTime);

/** The playing track, or undefined when the queue is empty. */
export const currentTrack = (s: PlayerState): Track | undefined => s.queue[s.index];
