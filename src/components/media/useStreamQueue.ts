import { useCallback, useEffect, useRef, useState } from "react";
import { COMMANDS } from "../../lib/commands";
import type { AudioQuality, StreamInfo, Track } from "../../lib/types";

/** Upper bound on tracks waiting in the queue. */
export const MAX_QUEUED = 10;

/** A resolved URL counts as stale this long before the provider's own expiry. */
const EXPIRY_MARGIN_SECS = 60;

export type QueueState = "pending" | "resolving" | "ready" | "failed";

export type QueuedTrack = {
  track: Track;
  state: QueueState;
  stream?: StreamInfo;
  error?: string;
};

const stale = (s: StreamInfo) => s.expiresAt != null && s.expiresAt - EXPIRY_MARGIN_SECS < Date.now() / 1000;

/**
 * Playback queue that lives in memory only. Stream URLs are resolved ahead of time, so the next
 * track starts without waiting on yt-dlp. Resolving is sequential on purpose, since every call
 * spawns a process and the provider throttles aggressive clients.
 */
export function useStreamQueue(quality: AudioQuality) {
  const [entries, setEntries] = useState<QueuedTrack[]>([]);
  const resolving = useRef(false);

  const patch = useCallback(
    (id: string, change: Partial<QueuedTrack>) =>
      setEntries((prev) => prev.map((e) => (e.track.id === id ? { ...e, ...change } : e))),
    [],
  );

  useEffect(() => {
    if (resolving.current) return;
    const next = entries.find((e) => e.state === "pending");
    if (!next) return;
    const id = next.track.id;
    resolving.current = true;
    patch(id, { state: "resolving" });
    COMMANDS.media.stream({ id, quality }).then(
      (stream) => {
        resolving.current = false;
        patch(id, { state: "ready", stream });
      },
      (e) => {
        resolving.current = false;
        patch(id, { state: "failed", error: String(e) });
      },
    );
  }, [entries, quality, patch]);

  const add = useCallback(
    (track: Track): "added" | "duplicate" | "full" => {
      if (entries.some((e) => e.track.id === track.id)) return "duplicate";
      if (entries.length >= MAX_QUEUED) return "full";
      setEntries((prev) => [...prev, { track, state: "pending" }]);
      return "added";
    },
    [entries],
  );

  const remove = useCallback((id: string) => setEntries((prev) => prev.filter((e) => e.track.id !== id)), []);
  const clear = useCallback(() => setEntries([]), []);

  /**
   * Removes an entry (the head by default) and returns it with a stream that is safe to play.
   * An entry that is not resolved yet or went stale is resolved on the spot.
   */
  const take = useCallback(
    async (id?: string) => {
      const entry = id ? entries.find((e) => e.track.id === id) : entries[0];
      if (!entry) return null;
      remove(entry.track.id);
      const ready = entry.state === "ready" && entry.stream && !stale(entry.stream);
      const stream = ready ? entry.stream! : await COMMANDS.media.stream({ id: entry.track.id, quality });
      return { track: entry.track, stream };
    },
    [entries, quality, remove],
  );

  return { entries, add, remove, clear, take };
}
