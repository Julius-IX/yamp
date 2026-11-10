import { useEffect, useState } from "react";
import { COMMANDS } from "./commands";
import { logger } from "./log";
import { isBoolean, useSetting } from "./settings";
import { isRadio } from "@/radio/station";
import { currentTrack, getTime, onSeek, usePlayer } from "@/player/store";

const log = logger("discord");

/** Whether this build can show a presence. False on a phone, and without a Discord application id. */
export function useDiscordAvailable() {
  const [available, setAvailable] = useState(false);
  useEffect(() => {
    COMMANDS.discord.available().then(setAvailable).catch(() => setAvailable(false));
  }, []);
  return available;
}

/**
 * Shows what plays as the Discord activity while the option is on. Paused or stopped clears it.
 * Mount once. A failed update is common, Discord is simply not running, so only the first
 * failure of a row is logged.
 */
export function useDiscordPresence() {
  const available = useDiscordAvailable();
  const [enabled] = useSetting("discordPresence", false, isBoolean);
  const state = usePlayer();
  const track = currentTrack(state);
  const live = !!track && state.playing && enabled && available;

  useEffect(() => {
    if (!available) return;
    let failed = false;
    const fail = (e: unknown) => {
      if (!failed) log.warn("couldn't update the presence:", e);
      failed = true;
    };
    const send = () => {
      if (!track) return;
      const { position, duration } = getTime();
      const startMs = Date.now() - position * 1000;
      const length = duration || track.durationSecs || 0;
      COMMANDS.discord
        .update({
          title: track.title,
          artist: track.artist || "Unknown artist",
          cover: track.thumbnail,
          startMs: Math.round(startMs),
          // a live stream has no end
          endMs: length > 0 && !isRadio(track.id) ? Math.round(startMs + length * 1000) : null,
        })
        .then(() => (failed = false))
        .catch(fail);
    };
    if (!live) {
      COMMANDS.discord.clear().catch(fail);
      return;
    }
    send();
    // a seek moves the bar, which Discord only learns from a new start time
    return onSeek(send);
  }, [available, live, track?.id]);
}
