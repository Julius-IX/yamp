import { useEffect, useRef, useState } from "react";
import { logger } from "@/lib/log";

const log = logger("browse");

/** Provider queries are slow, so a result is reused for this long when a screen is mounted again. */
const TTL_MS = 10 * 60_000;
/** Every query starts a process on desktop, so only a few run at once and the rest wait. */
const MAX_RUNNING = 3;

const cache = new Map<string, { at: number; value: unknown }>();

const fresh = <T>(key: string): T | undefined => {
  const hit = cache.get(key);
  return hit && Date.now() - hit.at < TTL_MS ? (hit.value as T) : undefined;
};

let running = 0;
const waiting: (() => void)[] = [];

/** A finished query hands its slot straight to the next one waiting, so the cap is never exceeded. */
async function gated<T>(job: () => Promise<T>): Promise<T> {
  if (running >= MAX_RUNNING) await new Promise<void>((resolve) => waiting.push(resolve));
  else running++;
  try {
    return await job();
  } finally {
    const next = waiting.shift();
    if (next) next();
    else running--;
  }
}

/** Forgets every result whose key starts with `prefix`, for data that just changed. */
export function invalidate(prefix: string) {
  for (const key of cache.keys()) if (key.startsWith(prefix)) cache.delete(key);
}

/**
 * Loads a value once and shares it. `undefined` means still loading, `null` as a key means
 * nothing to load yet. A failed load gives `fallback` and is not cached, so the next mount
 * tries again. A changed key loads again, so it should name everything the result depends on.
 */
export function useCached<T>(key: string | null, load: () => Promise<T>, fallback: T): T | undefined {
  const [value, setValue] = useState<T | undefined>(() => (key ? fresh<T>(key) : undefined));
  // the callers pass new closures on every render, only the key says whether to load again
  const loader = useRef(load);
  loader.current = load;
  const failed = useRef(fallback);
  failed.current = fallback;

  useEffect(() => {
    if (!key) return setValue(undefined);
    const hit = fresh<T>(key);
    if (hit !== undefined) return setValue(hit);
    let stale = false;
    setValue(undefined);
    gated(() => loader.current())
      .then((loaded) => {
        cache.set(key, { at: Date.now(), value: loaded });
        if (!stale) setValue(loaded);
      })
      .catch((e) => {
        log.warn(`couldn't load "${key}":`, e);
        if (!stale) setValue(failed.current);
      });
    return () => {
      stale = true;
    };
  }, [key]);

  return value;
}
