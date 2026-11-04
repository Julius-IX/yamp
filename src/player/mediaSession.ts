import { useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { currentTrack, getTime, player, usePlayer } from "./store";

const SEEK_STEP_SECS = 10;

/**
 * Publishes the playing track to the system. The window title is what taskbars show on desktop,
 * and the Media Session is what media widgets, lock screens and keyboard media keys use where the
 * webview exposes it. Mount it once.
 */
export function usePlayerMediaSession() {
  const state = usePlayer();
  const track = currentTrack(state);

  useEffect(() => {
    const label = track ? [track.title, track.artist].filter(Boolean).join(" · ") : "yamp";
    try {
      getCurrentWindow()
        .setTitle(label)
        .catch(() => {});
    } catch {
      // no native window to retitle
    }
    if (!("mediaSession" in navigator)) return;
    navigator.mediaSession.metadata = track
      ? new MediaMetadata({
          title: track.title,
          artist: track.artist,
          album: track.album ?? "",
          artwork: track.thumbnail ? [{ src: track.thumbnail }] : [],
        })
      : null;
  }, [track?.id]);

  useEffect(() => {
    if (!("mediaSession" in navigator)) return;
    const session = navigator.mediaSession;
    session.playbackState = !track ? "none" : state.playing ? "playing" : "paused";
    // a position state without a length is rejected, and the provider does not always know it
    const duration = getTime().duration || track?.durationSecs || 0;
    if (duration > 0) {
      try {
        session.setPositionState({ duration, position: Math.min(getTime().position, duration), playbackRate: 1 });
      } catch {
        // some webviews only accept the call while audio is active
      }
    }
  }, [track?.id, state.playing]);

  // the handlers read the store when they run, so they are registered once
  useEffect(() => {
    if (!("mediaSession" in navigator)) return;
    const session = navigator.mediaSession;
    const seekBy = (delta: number) => player.seek(Math.max(0, getTime().position + delta));
    const handlers: [MediaSessionAction, MediaSessionActionHandler][] = [
      ["play", () => player.togglePlay()],
      ["pause", () => player.togglePlay()],
      ["previoustrack", () => player.prev()],
      ["nexttrack", () => player.next()],
      ["seekbackward", () => seekBy(-SEEK_STEP_SECS)],
      ["seekforward", () => seekBy(SEEK_STEP_SECS)],
      ["seekto", (d) => d.seekTime != null && player.seek(d.seekTime)],
    ];
    for (const [action, handler] of handlers) {
      try {
        session.setActionHandler(action, handler);
      } catch {
        // an action the webview does not know is skipped
      }
    }
    return () => {
      for (const [action] of handlers) {
        try {
          session.setActionHandler(action, null);
        } catch {
          // same as above
        }
      }
    };
  }, []);
}
