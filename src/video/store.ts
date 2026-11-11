import { useSyncExternalStore } from "react";
import type { Track } from "@/lib/types";
import { currentTrack, getPlayer, player } from "@/player/store";

/**
 * Whether the video view is open. A video is a song like any other, the player plays its audio
 * and this view only shows the picture beside it, so closing it never stops anything. Kept
 * outside of React like the player, so any shelf can open it without the modal being part of
 * its tree.
 */
let open = false;
const listeners = new Set<() => void>();

const set = (next: boolean) => {
  if (open === next) return;
  open = next;
  listeners.forEach((l) => l());
};

export const video = {
  /** Plays `tracks` from `index` unless that track already plays, and shows its picture. */
  watch(tracks: Track[], index = 0) {
    const track = tracks[index];
    if (track && currentTrack(getPlayer())?.id !== track.id) player.play(tracks, index);
    set(true);
  },
  close() {
    set(false);
  },
};

const subscribe = (cb: () => void) => (listeners.add(cb), () => void listeners.delete(cb));

export const useVideoOpen = () => useSyncExternalStore(subscribe, () => open);
