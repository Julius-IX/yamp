import { useSyncExternalStore } from "react";
import type { Track } from "@/lib/types";
import { player } from "@/player/store";

/**
 * The video that is open, if any. Kept outside of React like the player, so any shelf can open
 * one without the modal being part of its tree.
 */
let current: Track | null = null;
const listeners = new Set<() => void>();

const set = (video: Track | null) => {
  current = video;
  listeners.forEach((l) => l());
};

export const video = {
  open(track: Track) {
    // the video carries its own sound, so the music must not play underneath it
    player.pause();
    set(track);
  },
  close() {
    set(null);
  },
};

const subscribe = (cb: () => void) => (listeners.add(cb), () => void listeners.delete(cb));

export const useVideo = () => useSyncExternalStore(subscribe, () => current);
