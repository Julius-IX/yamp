import { useEffect, useSyncExternalStore } from "react";
import { listen } from "@tauri-apps/api/event";
import { COMMANDS } from "./commands";
import { logger } from "./log";
import type { DownloadJob, Track } from "./types";

const log = logger("local");

/** Backend events, see `library.rs` and `yt-api`. */
const DOWNLOADED_EVENT = "library:downloaded";
const LOCAL_EVENT = "library:local";
const JOB_EVENT = "yt-api:download";

type LocalState = {
  /** Downloaded in the app. */
  downloaded: ReadonlySet<string>;
  /** Downloads that are queued or running. */
  busy: ReadonlySet<string>;
  /** Songs in the local playlist, downloads and files of the added folders together. */
  count: number;
  /** Raised by every reload, so a list of local songs knows when to read itself again. */
  version: number;
};

let state: LocalState = { downloaded: new Set(), busy: new Set(), count: 0, version: 0 };
let started = false;
/** A scan of a large collection is a long walk, so returning to the app twice in a row scans once. */
const RESCAN_GAP_MS = 15_000;
let lastRescan = 0;
const listeners = new Set<() => void>();

const publish = (patch: Partial<LocalState>) => {
  state = { ...state, ...patch };
  listeners.forEach((l) => l());
};

const without = (set: ReadonlySet<string>, id: string) => {
  const next = new Set(set);
  next.delete(id);
  return next;
};

/** Reloads what is downloaded and how many local songs there are. */
export const refreshLocal = () =>
  Promise.all([COMMANDS.downloads.list(), COMMANDS.local.count()])
    .then(([records, count]) => {
      const downloaded = new Set(records.map((r) => r.trackId));
      // a download that landed is no longer in progress
      const busy = new Set([...state.busy].filter((id) => !downloaded.has(id)));
      publish({ downloaded, busy, count, version: state.version + 1 });
    })
    .catch((e) => log.error("couldn't load the local songs:", e));

/**
 * Ids that came from a folder of the user start with this, see `store::local` in the backend.
 * They are on the device already, so there is nothing to download for them.
 */
export const isLocalFile = (id: string) => id.startsWith("local-");

/** The metadata is saved with the job, so the song can be listed once the file has landed. */
export async function downloadTrack(track: Track) {
  if (isLocalFile(track.id) || state.downloaded.has(track.id) || state.busy.has(track.id)) return;
  publish({ busy: new Set(state.busy).add(track.id) });
  try {
    await COMMANDS.downloads.enqueue({ track });
  } catch (e) {
    log.error(`couldn't download ${track.id}:`, e);
    publish({ busy: without(state.busy, track.id) });
  }
}

/** Deletes the file from this device. The song stays in the playlists and likes it is part of. */
export const removeDownload = (trackId: string) => removeDownloads([trackId]);

/**
 * Deletes the files of several songs and reloads once at the end. A song that fails does not
 * stop the rest, and the reload shows which ones are gone.
 */
export async function removeDownloads(trackIds: string[]) {
  for (const trackId of trackIds) {
    try {
      await COMMANDS.downloads.remove({ trackId });
    } catch (e) {
      log.error(`couldn't remove the download of ${trackId}:`, e);
    }
  }
  await refreshLocal();
}

/** The ids of the ones given that are downloaded. */
export const downloadedAmong = (ids: string[]) => ids.filter((id) => state.downloaded.has(id));

/** Subscribes once for the whole app, the events outlive any screen. */
function start() {
  if (started) return;
  started = true;
  void refreshLocal();
  void listen<string>(DOWNLOADED_EVENT, () => void refreshLocal());
  void listen(LOCAL_EVENT, () => void refreshLocal());
  // Folders on shared storage of a phone do not report files that other apps put there, and a
  // desktop can miss a change while asleep. Coming back to the app is the cue to look again. A
  // scan that finds nothing new is silent, so doing this often costs only a walk of the folders.
  const lookAgain = () => {
    if (document.visibilityState !== "visible" || Date.now() - lastRescan < RESCAN_GAP_MS) return;
    lastRescan = Date.now();
    COMMANDS.local.rescan().catch((e) => log.warn("couldn't scan the folders:", e));
  };
  document.addEventListener("visibilitychange", lookAgain);
  window.addEventListener("focus", lookAgain);
  void listen<DownloadJob>(JOB_EVENT, (e) => {
    const { id, state: job } = e.payload;
    // a finished job is not handled here, the event of the library tells when its file is in place
    if ((job === "failed" || job === "cancelled") && state.busy.has(id)) {
      publish({ busy: without(state.busy, id) });
    }
  });
}

const subscribe = (cb: () => void) => (listeners.add(cb), () => void listeners.delete(cb));

/** The first component that asks starts the loading, later ones reuse it. */
function useStarted() {
  useEffect(start, []);
}

export type DownloadStatus = "none" | "busy" | "done";

/** One status per row. A primitive keeps a row from rendering again for the changes of other songs. */
export function useDownloadStatus(trackId: string): DownloadStatus {
  useStarted();
  return useSyncExternalStore(subscribe, () =>
    state.downloaded.has(trackId) ? "done" : state.busy.has(trackId) ? "busy" : "none",
  );
}

/** How many of the songs are downloaded. A number keeps a bar from rendering again for other songs. */
export function useDownloadedCount(ids: string[]) {
  useStarted();
  return useSyncExternalStore(subscribe, () => downloadedAmong(ids).length);
}

/** How many songs the local playlist holds. */
export function useLocalCount() {
  useStarted();
  return useSyncExternalStore(subscribe, () => state.count);
}

/** Changes whenever the local songs may have changed, which is the cue for a list to load again. */
export function useLocalVersion() {
  useStarted();
  return useSyncExternalStore(subscribe, () => state.version);
}
