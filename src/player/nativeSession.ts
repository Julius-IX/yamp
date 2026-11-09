import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { COMMANDS } from "@/lib/commands";
import { artworkCandidates, isLocalCover } from "@/lib/artwork";
import { logger } from "@/lib/log";
import { IS_IOS } from "@/util/platform";
import { currentTrack, getPlayer, getTime, onSeek, player, usePlayer } from "./store";

const log = logger("session");

/** Matches `Action` in the `yamp-session` crate. */
type SessionAction =
  | { action: "play" | "pause" | "toggle" | "next" | "previous" | "stop" }
  | { action: "seekBy" | "seekTo"; secs: number };

/** The size the system widgets are given, they draw it small but a lock screen draws it large. */
const COVER_PX = 512;

function publish() {
  const state = getPlayer();
  const track = currentTrack(state);
  const time = getTime();
  COMMANDS.session
    .update({
      title: track?.title ?? null,
      artist: track?.artist ?? "",
      album: track?.album ?? null,
      // the system fetches the cover itself, a cached file of the app is out of its reach
      cover: isLocalCover(track?.thumbnail) ? null : (artworkCandidates(track?.thumbnail, COVER_PX)[0] ?? null),
      // the provider does not always know the length before the stream is open
      durationSecs: time.duration || track?.durationSecs || null,
      positionSecs: time.position,
      // a track change pauses and resolves the next stream for a while, which still counts as playing
      playing: state.playing || state.loading,
    })
    .catch((e) => log.warn("couldn't publish the playing track:", e));
}

function handle(a: SessionAction) {
  const { playing } = getPlayer();
  switch (a.action) {
    // a headset says which one it means, acting on the wrong one would flip the state back
    case "play":
      return playing || player.togglePlay();
    case "pause":
      return playing && player.togglePlay();
    case "toggle":
      return player.togglePlay();
    case "next":
      return player.next();
    case "previous":
      return player.prev();
    case "stop":
      return player.pause();
    case "seekBy":
      return player.seek(Math.max(0, getTime().position + a.secs));
    case "seekTo":
      return player.seek(a.secs);
  }
}

/**
 * Publishes the playing track to the system (media keys, headsets, lock screen, quick settings)
 * and the window title, and acts on what the system asks for. Mount it once.
 */
export function useNativeSession() {
  const state = usePlayer();
  const track = currentTrack(state);

  // The position is read when it is published, so only the changes that move it matter.
  useEffect(() => {
    if (IS_IOS) return;
    publish();
  }, [track?.id, state.playing, state.loading]);

  useEffect(() => {
    if (IS_IOS) return;
    return onSeek(publish);
  }, []);

  useEffect(() => {
    if (IS_IOS) return;
    const stop = listen<SessionAction>("yamp-session:action", (e) => handle(e.payload));
    return () => void stop.then((off) => off());
  }, []);
}
