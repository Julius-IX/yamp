import { useEffect, useState } from "react";
import { COMMANDS } from "./commands";
import { logger } from "./log";
import { isBoolean, useSetting } from "./settings";
import { isRadio } from "@/radio/station";
import { isLocalFile } from "./local";
import { currentTrack, getTime, onSeek, usePlayer } from "@/player/store";

const log = logger("discord");

/** The page of a track on YouTube Music, which the card links to. A local file or a station has none. */
const trackUrl = (id: string) =>
  isRadio(id) || isLocalFile(id) ? null : `https://music.youtube.com/watch?v=${encodeURIComponent(id)}`;

/** There is no id of the artist at hand, so the link is a search for the name. */
const artistUrl = (artist: string) => `https://music.youtube.com/search?q=${encodeURIComponent(artist)}`;

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
      const url = trackUrl(track.id);
      COMMANDS.discord
        .update({
          title: track.title,
          artist: track.artist || "Unknown artist",
          cover: track.thumbnail,
          album: track.album ?? null,
          startMs: Math.round(startMs),
          // a live stream has no end
          endMs: length > 0 && !isRadio(track.id) ? Math.round(startMs + length * 1000) : null,
          url,
          artistUrl: url && track.artist ? artistUrl(track.artist) : null,
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
