import { useEffect, useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { Now } from "./Player";

/**
 * Publishes the playing track to the system. The window title is what taskbars show on desktop,
 * and the Media Session metadata is what media widgets and lock screens read where the webview
 * exposes it. Play, pause and seeking need no handlers, the browser routes them to the audio
 * element on its own.
 */
export function useMediaSession(now: Now | null, onNext: () => void) {
  useEffect(() => {
    const label = now ? [now.title, now.artist].filter(Boolean).join(" · ") : "yamp";
    try {
      getCurrentWindow()
        .setTitle(label)
        .catch(() => {});
    } catch {
      // no native window to retitle
    }
    if (!("mediaSession" in navigator)) return;
    navigator.mediaSession.metadata = now
      ? new MediaMetadata({
          title: now.title,
          artist: now.artist ?? "",
          artwork: now.artwork ? [{ src: now.artwork }] : [],
        })
      : null;
  }, [now?.title, now?.artist, now?.artwork]);

  // the handler is registered once and reads the latest callback, so renders do not re-register it
  const next = useRef(onNext);
  next.current = onNext;
  useEffect(() => {
    if (!("mediaSession" in navigator)) return;
    navigator.mediaSession.setActionHandler("nexttrack", () => next.current());
    return () => navigator.mediaSession.setActionHandler("nexttrack", null);
  }, []);
}
