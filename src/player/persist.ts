import { getSetting, isNumber, setSetting } from "@/lib/settings";
import type { Track } from "@/lib/types";

/**
 * The queue and position that survive a restart, so the app opens on the last played track,
 * paused. It is one setting because the pieces only make sense together.
 */
export type Saved = {
  queue: Track[];
  /** The order before shuffling. Only saved while shuffle is on, otherwise it equals the queue. */
  original: Track[] | null;
  /** The playlist or album that was played, so shuffle still covers it after a restart. Only saved while shuffle is on. */
  source?: Track[] | null;
  index: number;
  position: number;
};

const KEY = "playback";
/** Autoplay can grow the queue without limit, and the settings file is read on every start. */
const MAX_SAVED = 100;
/** Tracks kept from before the playing one, so previous still has somewhere to go. */
const KEEP_BEHIND = 20;
/** The source is a whole playlist, so it gets more room than the queue. */
const MAX_SOURCE = 500;

const isSaved = (v: unknown): v is Saved => {
  const s = v as Saved | null;
  return (
    !!s &&
    Array.isArray(s.queue) &&
    s.queue.every((t) => typeof t?.id === "string") &&
    (s.source == null || (Array.isArray(s.source) && s.source.every((t) => typeof t?.id === "string"))) &&
    isNumber(s.index) &&
    isNumber(s.position) &&
    s.index >= 0 &&
    s.index < s.queue.length
  );
};

export function loadPlayback(): Saved | null {
  const raw = getSetting<unknown>(KEY, null);
  return isSaved(raw) ? raw : null;
}

export function savePlayback(
  queue: Track[],
  original: Track[],
  source: Track[],
  shuffle: boolean,
  index: number,
  position: number,
) {
  if (index < 0 || queue.length === 0) return setSetting(KEY, null);
  const kept = shuffle ? source.slice(0, MAX_SOURCE) : null;
  if (queue.length <= MAX_SAVED) {
    return setSetting<Saved>(KEY, { queue, original: shuffle ? original : null, source: kept, index, position });
  }
  // a trimmed queue cannot be put back into its unshuffled order
  const from = Math.max(0, index - KEEP_BEHIND);
  setSetting<Saved>(KEY, { queue: queue.slice(from, from + MAX_SAVED), original: null, source: kept, index: index - from, position });
}
