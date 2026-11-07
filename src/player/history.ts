import { getSetting, setSetting, useSetting } from "@/lib/settings";
import type { Track } from "@/lib/types";

/**
 * The tracks that were played, newest first. The backend keeps no play history, so it lives in
 * the settings next to the saved queue. Home reads it for the recently played shelf and to pick
 * what recommendations are based on.
 */
const KEY = "history";
/** The settings file is read on every start, so the list stays short. */
const MAX = 30;

const isHistory = (v: unknown): v is Track[] =>
  Array.isArray(v) && v.every((t) => typeof t?.id === "string");

// A stable fallback, a new array on every read would make the hook re-render forever.
const EMPTY: Track[] = [];

let lastId: string | undefined;

/** Moves the track to the front. The same track playing again, like after a pause, is not a new entry. */
export function recordPlayed(track: Track) {
  if (track.id === lastId) return;
  lastId = track.id;
  const rest = readHistory().filter((t) => t.id !== track.id);
  setSetting(KEY, [track, ...rest].slice(0, MAX));
}

/** A snapshot, for code that must not re-run when a track starts playing. */
export const readHistory = () => getSetting(KEY, EMPTY, isHistory);

export const useHistory = () => useSetting<Track[]>(KEY, EMPTY, isHistory)[0];
