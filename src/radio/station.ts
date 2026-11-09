import type { Track } from "@/lib/types";

/**
 * A station as the screens use it, reduced from what the Radio Browser lists. A station is
 * played as a `Track` so the queue, the player bar and the lock screen need no second kind of
 * item. The prefix on the track id is what tells the player to treat it as a live stream.
 */
export type Station = {
  /** The Radio Browser's own id of the station. */
  id: string;
  name: string;
  country: string;
  /** Zero when the station does not say. */
  bitrate: number;
  tags: string[];
  favicon: string | null;
};

const PREFIX = "radio:";

export const isRadio = (trackId: string) => trackId.startsWith(PREFIX);

export const stationId = (trackId: string) => trackId.slice(PREFIX.length);

/** "The Netherlands · 128 kbps", with the parts a station leaves out dropped. */
export const stationLine = (s: Station) =>
  [s.country, s.bitrate > 0 ? `${s.bitrate} kbps` : null].filter(Boolean).join(" · ");

export const stationTrack = (s: Station): Track => ({
  id: PREFIX + s.id,
  title: s.name,
  artist: stationLine(s),
  album: null,
  durationSecs: null,
  thumbnail: s.favicon,
  releaseYear: null,
});
