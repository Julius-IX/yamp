import { COMMANDS } from "@/lib/commands";
import { logger } from "@/lib/log";
import type { Track } from "@/lib/types";
import { player } from "@/player/store";

const log = logger("library");

export const shuffled = (tracks: Track[]) => {
  const out = [...tracks];
  for (let i = out.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [out[i], out[j]] = [out[j], out[i]];
  }
  return out;
};

/** The liked songs, newest first. The count is the limit, since playing needs all of them. */
export async function loadLiked(): Promise<Track[]> {
  const count = await COMMANDS.likes.count();
  return count > 0 ? COMMANDS.likes.list({ limit: count, offset: 0 }) : [];
}

/** The local songs, newest first. The count is the limit, since playing needs all of them. */
export async function loadLocal(): Promise<Track[]> {
  const count = await COMMANDS.local.count();
  return count > 0 ? COMMANDS.local.tracks({ limit: count, offset: 0 }) : [];
}

/** A list that is only loaded when it is played, so a card does not hold its tracks. */
async function run(load: () => Promise<Track[]>, how: "play" | "queue", what: string) {
  try {
    const tracks = await load();
    if (tracks.length === 0) return;
    if (how === "play") player.play(tracks, 0);
    else player.enqueue(tracks, "end");
  } catch (e) {
    log.warn(`couldn't ${how} ${what}:`, e);
  }
}

export const playPlaylist = (id: string) => run(() => COMMANDS.playlists.tracks({ id }), "play", "a playlist");
export const queuePlaylist = (id: string) => run(() => COMMANDS.playlists.tracks({ id }), "queue", "a playlist");
export const playLiked = () => run(loadLiked, "play", "the liked songs");
export const queueLiked = () => run(loadLiked, "queue", "the liked songs");
export const playLocal = () => run(loadLocal, "play", "the local songs");
export const queueLocal = () => run(loadLocal, "queue", "the local songs");
