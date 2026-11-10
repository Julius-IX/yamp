// Errors that the person should hear about. Everything logged at the error level, by the backend
// or by the frontend, ends up as a `log:entry` event, so this one listener sees all of them.
// Warnings and below stay in the log view, they are not worth interrupting for.
import { useSyncExternalStore } from "react";
import { listen } from "@tauri-apps/api/event";
import type { LogEntry } from "./types";

export type ErrorNotice = {
  id: number;
  message: string;
  /** How many times the same message came in while it was shown. */
  count: number;
};

/** The notices on screen at once. An older one is dropped for a newer one beyond this. */
const MAX_SHOWN = 3;
const SHOWN_MS = 8000;
/** A message is cut here, the whole of it is in the log view. */
const MAX_LENGTH = 160;

let notices: readonly ErrorNotice[] = [];
const listeners = new Set<() => void>();
const timers = new Map<number, number>();

function publish(next: readonly ErrorNotice[]) {
  notices = next;
  listeners.forEach((l) => l());
}

export function dismissError(id: number) {
  window.clearTimeout(timers.get(id));
  timers.delete(id);
  publish(notices.filter((n) => n.id !== id));
}

/** The first line only, a stack trace would fill the screen. */
const summarize = (message: string) => {
  const line = message.split("\n")[0].trim();
  return line.length > MAX_LENGTH ? `${line.slice(0, MAX_LENGTH)}...` : line;
};

function show(entry: LogEntry) {
  const message = summarize(entry.message);
  const same = notices.find((n) => n.message === message);
  // a repeat counts up and keeps the notice on screen a while longer, instead of stacking copies
  const id = same?.id ?? entry.id;
  window.clearTimeout(timers.get(id));
  timers.set(id, window.setTimeout(() => dismissError(id), SHOWN_MS));
  if (same) {
    publish(notices.map((n) => (n.id === id ? { ...n, count: n.count + 1 } : n)));
    return;
  }
  const next = [...notices, { id, message, count: 1 }];
  next.slice(0, -MAX_SHOWN).forEach((n) => dismissError(n.id));
  publish(next.slice(-MAX_SHOWN));
}

let started = false;

/** Starts listening. Called once from the layout, a second call does nothing. */
export function watchErrors() {
  if (started) return;
  started = true;
  void listen<LogEntry>("log:entry", (e) => {
    if (e.payload.level === "error") show(e.payload);
  });
}

export const useErrorNotices = () =>
  useSyncExternalStore((cb) => (listeners.add(cb), () => void listeners.delete(cb)), () => notices);
