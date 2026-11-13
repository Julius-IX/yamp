import { useEffect } from "react";
import { resolveArtwork } from "@/lib/artwork";
import { IS_IOS, IS_LINUX } from "@/util/platform";
import { currentTrack, getTime, player, usePlayer } from "./store";

const SEEK_STEP_SECS = 10;

/**
 * The webview's own Media Session. iOS gets all of it published to the system, since WKWebView
 * does. WebKitGTK on Linux publishes a player of its own next to the native one and names it
 * after the page title, so only the metadata is set there. Without it, widgets that pick that
 * player show the app name and not the track. Everywhere else the webview does not publish,
 * which `useNativeSession` covers. Mount it once.
 */
export function usePlayerMediaSession() {
  const state = usePlayer();
  const track = currentTrack(state);

  useEffect(() => {
    if (!(IS_IOS || IS_LINUX) || !("mediaSession" in navigator)) return;
    navigator.mediaSession.metadata = track
      ? new MediaMetadata({
          title: track.title,
          artist: track.artist,
          album: track.album ?? "",
          artwork: [{ src: resolveArtwork(track.thumbnail) ?? "" }].filter((a) => a.src),
        })
      : null;
  }, [track?.id]);

  useEffect(() => {
    if (!IS_IOS || !("mediaSession" in navigator)) return;
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
    if (!IS_IOS || !("mediaSession" in navigator)) return;
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
