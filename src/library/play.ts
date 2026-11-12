import { applyOrder, savedOrder } from "@/lib/arrange";
import { COMMANDS } from "@/lib/commands";
import { logger } from "@/lib/log";
import type { Track } from "@/lib/types";
import { beginPick, player, stillWanted } from "@/player/store";

const log = logger("library");

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
  const pick = how === "play" ? beginPick() : 0;
  try {
    const tracks = await load();
    if (tracks.length === 0) return;
    if (how === "play") {
      // something else was picked while this loaded, and that is what should play
      if (stillWanted(pick)) player.play(tracks, 0);
    } else {
      player.enqueue(tracks, "end");
    }
  } catch (e) {
    log.warn(`couldn't ${how} ${what}:`, e);
  }
}

export const playPlaylist = (id: string) => run(() => COMMANDS.playlists.tracks({ id }), "play", "a playlist");
export const queuePlaylist = (id: string) => run(() => COMMANDS.playlists.tracks({ id }), "queue", "a playlist");
// played from a card, so they follow the order the list has on its own screen
const arranged = (load: () => Promise<Track[]>, slot: "liked" | "local") => async () =>
  applyOrder(await load(), await savedOrder(slot));

export const playLiked = () => run(arranged(loadLiked, "liked"), "play", "the liked songs");
export const queueLiked = () => run(arranged(loadLiked, "liked"), "queue", "the liked songs");
export const playLocal = () => run(arranged(loadLocal, "local"), "play", "the local songs");
export const queueLocal = () => run(arranged(loadLocal, "local"), "queue", "the local songs");
