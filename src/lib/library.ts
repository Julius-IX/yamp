import { useEffect, useSyncExternalStore } from "react";
import { COMMANDS } from "./commands";
import { logger } from "./log";
import type { LibraryPlaylist, Track } from "./types";

const log = logger("library");

/**
 * Every playlist of the library, shared by the sidebar, the library screen and the add menu. One
 * query fills it and each change made through this module reloads it, so a list never asks the
 * backend on its own. It stays null until the first answer, which tells a library that is still
 * loading from an empty one.
 */
let playlists: readonly LibraryPlaylist[] | null = null;
let started = false;
const listeners = new Set<() => void>();

/** Reloads the list. Needed after a sync pulled changes, or when a screen saved something by itself. */
export const refreshPlaylists = () =>
  COMMANDS.playlists
    .list()
    .then(async (list) => {
      await loadCovers(list);
      playlists = list;
      listeners.forEach((l) => l());
    })
    .catch((e) => {
      log.error("couldn't load the playlists:", e);
      // without this the screens would wait for an answer that is not coming
      if (playlists === null) {
        playlists = [];
        listeners.forEach((l) => l());
      }
    });

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

/**
 * A saved album is stored as a playlist under the id the provider gave it, while the id of a
 * playlist made in the app is a random uuid. Any other id therefore belongs to an album.
 */
export const isSavedAlbum = (p: LibraryPlaylist) => !UUID.test(p.id);

/** Each of these returns whether it worked, a failure is logged and leaves the list as it was. */
async function change<T>(what: string, run: () => Promise<T>): Promise<T | null> {
  try {
    const result = await run();
    await refreshPlaylists();
    return result;
  } catch (e) {
    log.error(`couldn't ${what}:`, e);
    return null;
  }
}

export const createPlaylist = (name: string, description: string | null) =>
  change("create a playlist", () => COMMANDS.playlists.create({ name, description }));

export const editPlaylist = async (id: string, name: string, description: string | null) =>
  (await change("edit a playlist", () => COMMANDS.playlists.update({ id, name, description }))) !== null;

export const deletePlaylist = async (id: string) =>
  (await change("delete a playlist", () => COMMANDS.playlists.delete({ id }))) !== null;

export const addToPlaylist = async (id: string, tracks: Track[]) =>
  (await change("add to a playlist", () => COMMANDS.playlists.addTracks({ id, tracks }))) !== null;

export const removeFromPlaylist = async (id: string, position: number) =>
  (await change("remove from a playlist", () => COMMANDS.playlists.removeAt({ id, position }))) !== null;

/**
 * The covers that the user set, as data URLs. They are read with the list, before it is shown,
 * so a card never draws the thumbnail first and swaps it a moment later. Each is kept with the
 * version it was read for, and only a cover whose version changed is read again.
 */
const customCovers = new Map<string, { version: number; url: string }>();

async function loadCovers(list: readonly LibraryPlaylist[]) {
  const wanted = new Map<string, number>();
  for (const p of list) if (p.coverVersion !== null) wanted.set(p.id, p.coverVersion);
  for (const id of customCovers.keys()) if (!wanted.has(id)) customCovers.delete(id);

  const stale = [...wanted].filter(([id, version]) => customCovers.get(id)?.version !== version);
  await Promise.all(
    stale.map(async ([id, version]) => {
      try {
        const url = await COMMANDS.playlists.cover({ id });
        if (url) customCovers.set(id, { version, url });
      } catch (e) {
        log.warn(`couldn't load the cover of ${id}:`, e);
      }
    }),
  );
}

/** The cover to draw for a playlist, the one the user set or else the thumbnail of its first track. */
export const coverOf = (p: LibraryPlaylist) => customCovers.get(p.id)?.url ?? p.cover;

/** The longest edge of what is sent. The backend crops and scales for good, this only keeps a photo from crossing the bridge at full size. */
const SEND_PX = 1024;

async function shrink(file: Blob): Promise<number[]> {
  const bitmap = await createImageBitmap(file);
  const scale = Math.min(1, SEND_PX / Math.max(bitmap.width, bitmap.height));
  const canvas = document.createElement("canvas");
  canvas.width = Math.round(bitmap.width * scale);
  canvas.height = Math.round(bitmap.height * scale);
  canvas.getContext("2d")?.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
  bitmap.close();
  const jpeg = await new Promise<Blob | null>((done) => canvas.toBlob(done, "image/jpeg", 0.92));
  if (!jpeg) throw new Error("the image could not be encoded");
  return Array.from(new Uint8Array(await jpeg.arrayBuffer()));
}

export const setPlaylistCover = async (id: string, file: Blob) =>
  (await change("set a playlist cover", async () => COMMANDS.playlists.setCover({ id, image: await shrink(file) }))) !== null;

export const removePlaylistCover = async (id: string) =>
  (await change("remove a playlist cover", () => COMMANDS.playlists.removeCover({ id }))) !== null;

const subscribe = (cb: () => void) => (listeners.add(cb), () => void listeners.delete(cb));

/** Null while the first load is running. The first component that asks starts it. */
export function usePlaylists() {
  useEffect(() => {
    if (started) return;
    started = true;
    void refreshPlaylists();
  }, []);
  return useSyncExternalStore(subscribe, () => playlists);
}
