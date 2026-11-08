import { useEffect, useSyncExternalStore } from "react";
import { COMMANDS } from "./commands";
import { logger } from "./log";
import type { Track } from "./types";

const log = logger("likes");

/**
 * Ids of every liked track, shared by everything that draws a heart. One query fills the set and
 * a toggle updates it right away, so a list never asks the backend once per row.
 */
let liked: ReadonlySet<string> = new Set();
let loaded = false;
const listeners = new Set<() => void>();

const publish = (next: ReadonlySet<string>) => {
  liked = next;
  listeners.forEach((l) => l());
};

const withId = (id: string, on: boolean) => {
  const next = new Set(liked);
  if (on) next.add(id);
  else next.delete(id);
  return next;
};

/** Reloads the ids. Needed after a sync pulled changes from another device. */
export const refreshLikes = () =>
  COMMANDS.likes
    .ids()
    .then((ids) => publish(new Set(ids)))
    .catch((e) => log.error("couldn't load the likes:", e));

/** The full track is sent on a like, the library keeps it so the track can be listed offline. */
export async function toggleLike(track: Track) {
  const was = liked.has(track.id);
  publish(withId(track.id, !was));
  try {
    if (was) await COMMANDS.likes.unlike({ trackId: track.id });
    else await COMMANDS.likes.like({ track });
  } catch (e) {
    log.error(`couldn't ${was ? "unlike" : "like"} ${track.id}:`, e);
    publish(withId(track.id, was));
  }
}

const subscribe = (cb: () => void) => (listeners.add(cb), () => void listeners.delete(cb));

/** The first component that asks fills the set, later ones reuse it. */
function useLoadedOnce() {
  useEffect(() => {
    if (loaded) return;
    loaded = true;
    refreshLikes();
  }, []);
}

export function useIsLiked(trackId: string | undefined) {
  useLoadedOnce();
  return useSyncExternalStore(subscribe, () => trackId !== undefined && liked.has(trackId));
}

/** How many tracks are liked. A screen that lists them reloads when it changes. */
export function useLikedCount() {
  useLoadedOnce();
  return useSyncExternalStore(subscribe, () => liked.size);
}
