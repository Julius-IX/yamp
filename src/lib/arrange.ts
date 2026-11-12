import { useEffect, useMemo, useSyncExternalStore } from "react";
import { COMMANDS } from "./commands";
import { logger } from "./log";

const log = logger("arrange");

/**
 * How the user arranged the lists of the library: the order of the playlists, the order of the
 * liked and local songs, and a note for the last two. These have no table of their own, so they
 * live in the shared key value store of the library link, see `values` in the backend, and
 * follow the user to other devices with the next sync. Local songs are listed by the ids they
 * have on each device, so an order that names a song a device does not have is skipped there.
 */
const PREFIX = "arrange:";

export type OrderSlot = "playlists" | "liked" | "local";
export type NoteSlot = "liked" | "local";

const orderKey = (slot: OrderSlot) => `${PREFIX}order:${slot}`;
const noteKey = (slot: NoteSlot) => `${PREFIX}note:${slot}`;

// null until the first answer, which tells a list that is still loading from one that was never arranged
let stored: ReadonlyMap<string, string> | null = null;
let started = false;
const listeners = new Set<() => void>();

const publish = (next: ReadonlyMap<string, string>) => {
  stored = next;
  listeners.forEach((l) => l());
};

/** Reloads what is stored. Needed after a sync pulled changes from another device. */
export const refreshArrangement = () =>
  COMMANDS.values
    .list({ prefix: PREFIX })
    .then((found) => publish(new Map(found.map((v) => [v.key, v.value]))))
    .catch((e) => {
      log.error("couldn't load the arrangement:", e);
      // without this the lists would wait for an answer that is not coming
      if (stored === null) publish(new Map());
    });

// shown at once and saved behind it, a failed save is logged and the next reload shows what was kept
function write(key: string, value: string) {
  publish(new Map(stored).set(key, value));
  COMMANDS.values.set({ key, value }).catch((e) => log.error(`couldn't save ${key}:`, e));
}

export const saveOrder = (slot: OrderSlot, ids: readonly string[]) => write(orderKey(slot), JSON.stringify(ids));

/** An empty note is stored as an empty text, which reads the same as no note. */
export const saveNote = (slot: NoteSlot, note: string) => write(noteKey(slot), note.trim());

const subscribe = (cb: () => void) => (listeners.add(cb), () => void listeners.delete(cb));

function useStored() {
  useEffect(() => {
    if (started) return;
    started = true;
    void refreshArrangement();
  }, []);
  return useSyncExternalStore(subscribe, () => stored);
}

function parseIds(text: string | undefined): readonly string[] {
  try {
    const found: unknown = text ? JSON.parse(text) : [];
    return Array.isArray(found) ? found.filter((x): x is string => typeof x === "string") : [];
  } catch {
    return [];
  }
}

/** The saved order for code that is not a component, such as playing a list from a card. */
export async function savedOrder(slot: OrderSlot) {
  if (stored === null) await refreshArrangement();
  return parseIds(stored?.get(orderKey(slot)));
}

/** The saved order of a list, empty when it was never arranged. Null while loading. */
export function useOrder(slot: OrderSlot) {
  const map = useStored();
  return useMemo(() => map && parseIds(map.get(orderKey(slot))), [map, slot]);
}

/** The note of a list, empty when there is none. Null while loading. */
export function useNote(slot: NoteSlot) {
  const map = useStored();
  return map && (map.get(noteKey(slot)) ?? "");
}

/**
 * Puts `items` in the saved order. Items the order does not know yet, such as a song liked
 * after the last arrangement, keep their own order and go first, where a new song belongs in
 * lists that start with the newest. Ids in the order that are gone are skipped.
 */
export function applyOrder<T extends { id: string }>(items: readonly T[], saved: readonly string[]): T[] {
  const rank = new Map(saved.map((id, i) => [id, i]));
  const fresh = items.filter((t) => !rank.has(t.id));
  const known = items.filter((t) => rank.has(t.id)).sort((a, b) => rank.get(a.id)! - rank.get(b.id)!);
  return [...fresh, ...known];
}
