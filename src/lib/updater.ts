// Finding and installing new versions. Desktop installs a signed bundle from the backend, which
// downloads it and reports progress. Android has no way to replace itself, so the newest release
// is looked up on GitHub and its APK is opened for the system to install. iOS is not handled.
import { useEffect, useState, useSyncExternalStore } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { IS_ANDROID, IS_IOS } from "@/util/platform";
import { COMMANDS } from "./commands";
import { logger } from "./log";
import { isBoolean, setSetting, useSetting } from "./settings";
import type { UpdateInfo, UpdateProgress } from "./types";

const log = logger("updater");

const REPO = "Julius-IX/yamp";
const PROGRESS_EVENT = "updater:progress";
/** Short enough to catch a release soon, long enough to stay out of the way of the startup. */
const FIRST_CHECK_MS = 15_000;
const CHECK_EVERY_MS = 6 * 60 * 60 * 1000;

/** A release newer than the running one. `url` is where Android gets the APK. */
export type Pending = UpdateInfo & { url?: string };

export type UpdateStatus =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "current" }
  | { kind: "failed" }
  | { kind: "found"; update: Pending }
  | { kind: "installing"; update: Pending; downloaded: number; total: number | null }
  // installed, the new version runs after a restart
  | { kind: "ready"; update: Pending };

let status: UpdateStatus = { kind: "idle" };
const listeners = new Set<() => void>();

function set(next: UpdateStatus) {
  status = next;
  listeners.forEach((l) => l());
}

export const useUpdateStatus = () =>
  useSyncExternalStore((cb) => (listeners.add(cb), () => void listeners.delete(cb)), () => status);

let availability: Promise<boolean> | null = null;

/** Whether this install can be updated from the app. Asked once, the answer does not change. */
export function updatesAvailable() {
  availability ??= IS_ANDROID
    ? Promise.resolve(true)
    : IS_IOS
      ? Promise.resolve(false)
      : COMMANDS.updater.available().catch(() => false);
  return availability;
}

export function useUpdatesAvailable() {
  const [available, setAvailable] = useState(false);
  useEffect(() => {
    void updatesAvailable().then(setAvailable);
  }, []);
  return available;
}

/** The numbers before any suffix, so `0.2.0-beta.1` compares as `0.2.0`. */
const parts = (v: string) => (/^v?(\d+)\.(\d+)\.(\d+)/.exec(v) ?? []).slice(1).map(Number);

function isNewer(remote: string, local: string) {
  const [a, b] = [parts(remote), parts(local)];
  for (let i = 0; i < 3; i++) {
    if ((a[i] ?? 0) !== (b[i] ?? 0)) return (a[i] ?? 0) > (b[i] ?? 0);
  }
  return false;
}

type Release = {
  tag_name: string;
  body: string | null;
  html_url: string;
  assets: { name: string; browser_download_url: string }[];
};

async function latestRelease(): Promise<Pending | null> {
  const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
    headers: { Accept: "application/vnd.github+json" },
  });
  if (!res.ok) throw new Error(`GitHub answered ${res.status}`);
  const release = (await res.json()) as Release;
  const version = release.tag_name.replace(/^v/, "");
  if (!isNewer(version, await getVersion())) return null;
  // a release without an APK still leads to its page, where one can be found
  const apk = release.assets.find((a) => a.name.endsWith(".apk"));
  return { version, notes: release.body, url: apk?.browser_download_url ?? release.html_url };
}

/**
 * Looks for a newer version. A check that runs by itself only logs a failure as a warning, since
 * being offline is normal. One that was asked for is an error, the person is waiting for it.
 */
export async function checkForUpdates(asked: boolean) {
  if (status.kind === "checking" || status.kind === "installing" || status.kind === "ready") return;
  set({ kind: "checking" });
  try {
    const update = IS_ANDROID ? await latestRelease() : await COMMANDS.updater.check();
    set(update ? { kind: "found", update } : { kind: "current" });
  } catch (e) {
    (asked ? log.error : log.warn)("couldn't check for updates:", e);
    set({ kind: "failed" });
  }
}

export async function installUpdate() {
  if (status.kind !== "found") return;
  const { update } = status;
  if (IS_ANDROID) {
    await openUrl(update.url ?? `https://github.com/${REPO}/releases/latest`).catch((e) =>
      log.error("couldn't open the download:", e),
    );
    return;
  }
  set({ kind: "installing", update, downloaded: 0, total: null });
  const stop = await listen<UpdateProgress>(PROGRESS_EVENT, (e) =>
    set({ kind: "installing", update, ...e.payload }),
  );
  try {
    await COMMANDS.updater.install();
    set({ kind: "ready", update });
  } catch (e) {
    log.error("couldn't install the update:", e);
    set({ kind: "found", update });
  } finally {
    stop();
  }
}

export const restartToUpdate = () =>
  COMMANDS.updater.restart().catch((e) => log.error("couldn't restart:", e));

const isString = (v: unknown): v is string => typeof v === "string";

/** The version the person chose not to be reminded of. A newer one is offered again. */
export const useSkippedVersion = () => useSetting("updateSkipped", "", isString)[0];
export const skipVersion = (version: string) => setSetting("updateSkipped", version);

/** Checks shortly after the start and then every few hours while the option is on. Mount once. */
export function useAutoUpdateCheck() {
  const [auto] = useSetting("updateCheck", true, isBoolean);
  useEffect(() => {
    if (!auto) return;
    let alive = true;
    let timer = 0;
    const run = async () => {
      if (!alive) return;
      if (await updatesAvailable()) await checkForUpdates(false);
      if (alive) timer = window.setTimeout(run, CHECK_EVERY_MS);
    };
    timer = window.setTimeout(run, FIRST_CHECK_MS);
    return () => {
      alive = false;
      window.clearTimeout(timer);
    };
  }, [auto]);
}
