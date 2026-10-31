// Frontend side of the central log. Entries go to the backend, which buffers them, forwards them
// to the platform log and emits them back, so the log view shows them next to backend entries.
import { COMMANDS } from "./commands";
import type { LogLevel } from "./types";

const CONSOLE: Record<LogLevel, "error" | "warn" | "info" | "debug"> = {
  error: "error",
  warn: "warn",
  info: "info",
  debug: "debug",
  trace: "debug",
};

/** Rejections are not always Error instances (DOMException, events), and JSON turns those into "{}". */
function describe(arg: object): string {
  const o = arg as { name?: unknown; message?: unknown; type?: unknown };
  const text = [o.name, o.message, o.type].filter((v) => typeof v === "string" && v).join(": ");
  let json = "";
  try {
    json = JSON.stringify(arg, Object.getOwnPropertyNames(arg));
  } catch {
    // circular structures
  }
  return `${arg.constructor?.name ?? "Object"} ${text} ${json}`.trim();
}

function format(arg: unknown): string {
  if (typeof arg === "string") return arg;
  if (arg instanceof Error) return arg.stack ?? `${arg.name}: ${arg.message}`;
  if (arg && typeof arg === "object") return describe(arg);
  try {
    return JSON.stringify(arg) ?? String(arg);
  } catch {
    // circular structures
    return String(arg);
  }
}

function send(level: LogLevel, target: string, args: unknown[]) {
  // devtools keep working as before, the backend copy is the one the log view reads
  console[CONSOLE[level]](`[${target}]`, ...args);
  COMMANDS.logging
    .push({ level, target, message: args.map(format).join(" ") })
    // a failing logger must never log again, that would loop
    .catch(() => {});
}

export type Logger = Record<LogLevel, (...args: unknown[]) => void>;

/** Logger tagged with a target, usually the name of a feature or module. */
export function logger(target: string): Logger {
  return {
    error: (...a) => send("error", target, a),
    warn: (...a) => send("warn", target, a),
    info: (...a) => send("info", target, a),
    debug: (...a) => send("debug", target, a),
    trace: (...a) => send("trace", target, a),
  };
}

export const log = logger("app");

/** Routes uncaught errors and rejected promises into the log. Call once at startup. */
export function captureGlobalErrors() {
  const err = logger("window");
  window.addEventListener("error", (e) => err.error(e.error ?? e.message));
  window.addEventListener("unhandledrejection", (e) => err.error("unhandled rejection:", e.reason));
}
