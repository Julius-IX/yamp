import { COMMANDS } from "@/lib/commands";
import { logger } from "@/lib/log";
import { getSetting, isBoolean, setSetting } from "@/lib/settings";
import type { Track } from "@/lib/types";

/**
 * Listening statistics. Time heard is added up per track and day, and one day of one device is
 * one value of the synced key value store, `stats:<day>:<device>`. A value is only ever written
 * by the device in its key, so two devices never overwrite each other and the days of all of them
 * add up when read, see `data.ts`.
 *
 * The track is copied into the day on purpose. A track that was only played is not in the library,
 * and another device could not show a title for it otherwise.
 */
const log = logger("stats");

export const STATS_PREFIX = "stats:";

export type TrackStat = {
  title: string;
  artist: string;
  thumbnail: string | null;
  /** Seconds heard. */
  secs: number;
  plays: number;
};

export type DayDoc = { tracks: Record<string, TrackStat> };

/** Stats are on unless the user turned them off. */
export const statsEnabled = () => getSetting("statsEnabled", true, isBoolean);

const FLUSH_EVERY_MS = 30_000;
/** A track counts as played once this much of it was heard, or half of a shorter one. */
const PLAY_AFTER_SECS = 30;
/** A longer step of the position is a seek or a stall, which is not listening. */
const MAX_STEP_SECS = 2;

const pad = (n: number) => String(n).padStart(2, "0");

/** The local calendar day, so an evening's listening stays on the day it was heard. */
export const dayOf = (ms: number) => {
  const d = new Date(ms);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
};

/** Named by this device in every key it writes. Not the id of the cloud link, which only exists once linked. */
function device() {
  const saved = getSetting<string>("statsDevice", "", (v): v is string => typeof v === "string");
  if (saved) return saved;
  const id = Math.random().toString(36).slice(2, 10);
  setSetting("statsDevice", id);
  return id;
}

const keyOf = (day: string) => `${STATS_PREFIX}${day}:${device()}`;

/** What was heard since the last write, by day and track. */
let pending = new Map<string, Map<string, TrackStat>>();
/** The days as last written, so a write does not have to read first. */
const days = new Map<string, DayDoc>();
let flushing = false;

type Meta = Pick<Track, "title" | "artist" | "thumbnail">;

function add(day: string, id: string, meta: Meta, secs: number, plays: number) {
  const tracks = pending.get(day) ?? new Map<string, TrackStat>();
  pending.set(day, tracks);
  const stat = tracks.get(id) ?? { title: meta.title, artist: meta.artist, thumbnail: meta.thumbnail, secs: 0, plays: 0 };
  stat.secs += secs;
  stat.plays += plays;
  tracks.set(id, stat);
}

// The track that is being counted, and how far into it the last update was.
let current: { id: string; last: number; heard: number; counted: boolean } | null = null;

/**
 * Called by every time update of the player. Only the steady advance of the position counts, a
 * seek or a stall is skipped, see `MAX_STEP_SECS`.
 */
export function trackListen(track: Track, position: number) {
  if (!statsEnabled()) return;
  // a track that starts over, like on repeat, is a new play
  if (!current || current.id !== track.id || (position < current.last - 1 && position < 2)) {
    current = { id: track.id, last: position, heard: 0, counted: false };
    return;
  }
  const step = position - current.last;
  current.last = position;
  if (step <= 0 || step > MAX_STEP_SECS) return;
  current.heard += step;

  const needed = Math.min(PLAY_AFTER_SECS, (track.durationSecs ?? Infinity) / 2);
  const plays = !current.counted && current.heard >= needed ? 1 : 0;
  if (plays) current.counted = true;
  add(dayOf(Date.now()), track.id, track, step, plays);
}

async function load(day: string): Promise<DayDoc> {
  const known = days.get(day);
  if (known) return known;
  const key = keyOf(day);
  const found = (await COMMANDS.values.list({ prefix: key })).find((v) => v.key === key);
  let doc: DayDoc = { tracks: {} };
  if (found) {
    try {
      doc = JSON.parse(found.value) as DayDoc;
    } catch {
      // a day that cannot be read is started over, it only holds counters
    }
  }
  days.set(day, doc);
  return doc;
}

/** Writes what was heard since the last call. A failed write puts it back for the next one. */
export async function flushStats() {
  if (flushing || pending.size === 0) return;
  flushing = true;
  const taken = pending;
  pending = new Map();
  try {
    for (const [day, tracks] of taken) {
      const doc = await load(day);
      for (const [id, d] of tracks) {
        const stat = (doc.tracks[id] ??= { title: d.title, artist: d.artist, thumbnail: d.thumbnail, secs: 0, plays: 0 });
        stat.secs += d.secs;
        stat.plays += d.plays;
      }
      await COMMANDS.values.set({ key: keyOf(day), value: JSON.stringify(doc) });
      taken.delete(day);
    }
  } catch (e) {
    log.warn("couldn't save the statistics:", e);
    // the days that were not written go back in front of what came in meanwhile
    for (const [day, tracks] of taken) {
      for (const [id, d] of tracks) add(day, id, d, d.secs, d.plays);
    }
    days.clear();
  } finally {
    flushing = false;
  }
}

/** Removes the statistics of every device. Other devices drop theirs at their next sync. */
export async function clearStats() {
  pending = new Map();
  days.clear();
  current = null;
  await COMMANDS.values.removePrefix({ prefix: STATS_PREFIX });
}

setInterval(() => void flushStats(), FLUSH_EVERY_MS);
// a closed or backgrounded app may be killed without a chance to write
document.addEventListener("visibilitychange", () => document.hidden && void flushStats());
window.addEventListener("pagehide", () => void flushStats());
