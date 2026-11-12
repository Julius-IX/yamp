import { useEffect, useSyncExternalStore } from "react";
import { IS_ANDROID, IS_IOS, IS_LINUX, IS_MAC, IS_WINDOWS } from "@/util/platform";
import { COMMANDS } from "./commands";
import { pullSettings } from "./cloudSettings";
import { refreshPlaylists } from "./library";
import { refreshArrangement } from "./arrange";
import { refreshLikes } from "./likes";
import { logger } from "./log";
import { getSetting, isNumber, setSetting } from "./settings";
import type { SyncProvider, SyncStatus } from "./types";

const log = logger("sync");

/** Sent on the window after a sync changed the library, for lists that keep their own copy. */
export const SYNCED_EVENT = "yamp:synced";

const SYNC_EVERY_MS = 5 * 60_000;
/** A return to the app does not sync again when one just ran. */
const FOCUS_GAP_MS = 60_000;
const COMPACT_EVERY_MS = 7 * 24 * 3600_000;

export type SyncUi = {
  status: SyncStatus | null;
  providers: SyncProvider[];
  /** False until the first answer, so an empty list can be told apart from one not asked for yet. */
  loaded: boolean;
  /** Id of the service whose sign in waits for the browser. */
  signingIn: string | null;
  syncing: boolean;
  /** When the last sync of this run finished. */
  lastSyncMs: number | null;
};

/**
 * Everything about the link to a cloud account, shared by the settings and the automatic sync.
 * The user only signs in. Syncing then runs by itself, at start, on a timer and when the app comes
 * back to the front.
 */
let state: SyncUi = { status: null, providers: [], loaded: false, signingIn: null, syncing: false, lastSyncMs: null };
const listeners = new Set<() => void>();

const set = (patch: Partial<SyncUi>) => {
  state = { ...state, ...patch };
  listeners.forEach((l) => l());
};

export const useSyncState = () =>
  useSyncExternalStore(
    (cb) => (listeners.add(cb), () => void listeners.delete(cb)),
    () => state,
  );

/** What this device is called in the list of devices of the account. */
function deviceName() {
  if (IS_ANDROID) return "Android";
  if (IS_IOS) return "iPhone";
  if (IS_WINDOWS) return "Windows PC";
  if (IS_MAC) return "Mac";
  return IS_LINUX ? "Linux PC" : "Desktop";
}

export async function loadSync() {
  try {
    const [status, providers] = await Promise.all([COMMANDS.sync.status(), COMMANDS.sync.providers()]);
    set({ status, providers, loaded: true });
  } catch (e) {
    log.error("couldn't read the sync state:", e);
  }
}

export async function signIn(provider: string) {
  if (state.signingIn) return;
  set({ signingIn: provider });
  try {
    set({ status: await COMMANDS.sync.signIn({ provider, deviceName: deviceName() }) });
    void syncNow();
  } catch (e) {
    // failures only go to the log, which the debugging section shows. A cancel is the user's own
    // choice and is not worth a line.
    if (!String(e).includes("cancelled")) log.error("sign in failed:", e);
  } finally {
    set({ signingIn: null });
  }
}

export const cancelSignIn = () => COMMANDS.sync.cancelSignIn().catch((e) => log.error("couldn't cancel:", e));

export async function signOut() {
  try {
    await COMMANDS.sync.disconnect();
    set({ lastSyncMs: null });
  } catch (e) {
    log.error("sign out failed:", e);
  }
  await loadSync();
}

export async function syncNow() {
  if (!state.status?.connected || state.syncing) return;
  set({ syncing: true });
  try {
    const report = await COMMANDS.sync.now();
    if (report.pulled > 0) {
      await Promise.all([refreshLikes(), refreshPlaylists(), refreshArrangement(), pullSettings()]);
      window.dispatchEvent(new Event(SYNCED_EVENT));
    }
    set({ lastSyncMs: Date.now() });
    await compactWhenDue();
  } catch (e) {
    log.warn("sync failed:", e);
  } finally {
    set({ syncing: false });
  }
}

/** The log of the account only shrinks when someone compacts it, which is meant to happen weekly. */
async function compactWhenDue() {
  const last = getSetting("lastCompact", 0, isNumber);
  if (Date.now() - last < COMPACT_EVERY_MS) return;
  await COMMANDS.sync.compact();
  setSetting("lastCompact", Date.now());
}

/** Mount once. Keeps the library in step without the user doing anything. */
export function useAutoSync() {
  useEffect(() => {
    let stopped = false;

    // the link is restored in the background at launch, so the first looks can find nothing yet
    (async () => {
      for (const wait of [0, 3000, 10000]) {
        if (wait) await new Promise((r) => setTimeout(r, wait));
        if (stopped) return;
        await loadSync();
        if (state.status?.connected) return void syncNow();
      }
    })();

    const timer = setInterval(() => void syncNow(), SYNC_EVERY_MS);
    const onVisible = () => {
      if (!document.hidden && Date.now() - (state.lastSyncMs ?? 0) > FOCUS_GAP_MS) void syncNow();
    };
    document.addEventListener("visibilitychange", onVisible);
    return () => {
      stopped = true;
      clearInterval(timer);
      document.removeEventListener("visibilitychange", onVisible);
    };
  }, []);
}
