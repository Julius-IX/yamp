import { useSyncExternalStore } from "react";
import { COMMANDS } from "./commands";
import { shrink } from "./library";
import { logger } from "./log";
import type { Track } from "./types";

const log = logger("covers");

/**
 * The covers that the user set for single tracks, by track id, as images that need no request.
 * They are read once before the first render like the settings, so a cover is never drawn as
 * the thumbnail first and swapped a moment later. Kept outside of React so any list can use it.
 */
let covers: ReadonlyMap<string, string> = new Map();
const listeners = new Set<() => void>();

const publish = (next: Map<string, string>) => {
  covers = next;
  listeners.forEach((l) => l());
};

export async function loadTrackCovers() {
  try {
    publish(new Map((await COMMANDS.tracks.covers()).map((c) => [c.trackId, c.url])));
  } catch (e) {
    log.error("couldn't load the track covers:", e);
  }
}

type Thumbed = Pick<Track, "id" | "thumbnail">;

/** The cover to draw for a track, the one the user set or else its own thumbnail. */
export const coverOfTrack = (track: Thumbed) => covers.get(track.id) ?? track.thumbnail;

const subscribe = (cb: () => void) => (listeners.add(cb), () => void listeners.delete(cb));

/** Whether the user set a cover for the track. */
export const useHasTrackCover = (id: string) => useSyncExternalStore(subscribe, () => covers.has(id));

/**
 * Like `coverOfTrack` for a component, which draws again when the cover of this track changes.
 * `thumbnail` is for a card that has the picture to show apart from the track.
 */
export function useTrackCover(track: Thumbed | null | undefined, thumbnail = track?.thumbnail) {
  const custom = useSyncExternalStore(subscribe, () => (track ? covers.get(track.id) : undefined));
  return custom ?? thumbnail;
}

/** Returns whether the cover was set. */
export async function setTrackCover(id: string, file: Blob) {
  try {
    const url = await COMMANDS.tracks.setCover({ id, image: await shrink(file) });
    publish(new Map(covers).set(id, url));
    return true;
  } catch (e) {
    log.error(`couldn't set the cover of ${id}:`, e);
    return false;
  }
}

export async function removeTrackCover(id: string) {
  try {
    await COMMANDS.tracks.removeCover({ id });
    const next = new Map(covers);
    next.delete(id);
    publish(next);
  } catch (e) {
    log.error(`couldn't remove the cover of ${id}:`, e);
  }
}
