import { useSyncExternalStore } from "react";
import { COMMANDS } from "./commands";
import { logger } from "./log";

/**
 * Saved UI state (theme, toggles, sizes), kept by the backend in `settings.json`. The settings
 * are loaded once before the first render, see `main.tsx`, so reading is synchronous.
 *
 * A key must also be listed in `SETTINGS` in `src-tauri/src/lib.rs`, the backend drops the rest.
 *
 *   const [sidebarW, setSidebarW] = useSetting("sidebarW", 240, isNumber);
 */
const log = logger("settings");

export const isNumber = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v);
export const isBoolean = (v: unknown): v is boolean => typeof v === "boolean";
export const isRecord = (v: unknown): v is Record<string, unknown> => typeof v === "object" && v !== null && !Array.isArray(v);

let cache: Record<string, unknown> = {};
const listeners = new Set<() => void>();
const changeListeners = new Set<(key: string, value: unknown) => void>();

/**
 * What is saved under the key as it is, or `undefined` when it was never set. `getSetting` cannot
 * tell that from a saved value that equals its fallback.
 */
export const rawSetting = (key: string): unknown => cache[key];

/** Called for a setting that was changed on this device. A value that came from the cloud is not reported. */
export const onSettingChange = (cb: (key: string, value: unknown) => void) => (changeListeners.add(cb), () => void changeListeners.delete(cb));

export async function loadSettings() {
  try {
    cache = await COMMANDS.settings.getSettings();
  } catch (e) {
    log.error("couldn't load the settings, using defaults:", e);
  }
}

/**
 * `valid` guards against a saved value that is not usable, like a hand-edited file or a setting
 * whose options changed. The fallback is returned instead.
 */
export function getSetting<T>(key: string, fallback: T, valid?: (v: unknown) => v is T): T {
  const v = cache[key];
  if (v === undefined) return fallback;
  return !valid || valid(v) ? (v as T) : fallback;
}

/**
 * Applies right away and saves in the background. A failed save is logged and not rolled back.
 * `fromCloud` marks a value that was taken over from another device, which must not be sent back.
 */
export function setSetting<T>(key: string, value: T, fromCloud = false) {
  cache = { ...cache, [key]: value };
  listeners.forEach((l) => l());
  if (!fromCloud) changeListeners.forEach((l) => l(key, value));
  COMMANDS.settings
    .setSetting({ key, value })
    .catch((e) => log.error(`couldn't save "${key}":`, e));
}

/** Like `useState`, but saved, and shared by everything that uses the same key. */
export function useSetting<T>(key: string, fallback: T, valid?: (v: unknown) => v is T) {
  const value = useSyncExternalStore(
    (cb) => (listeners.add(cb), () => void listeners.delete(cb)),
    () => getSetting(key, fallback, valid),
  );
  return [value, (v: T) => setSetting(key, v)] as const;
}
