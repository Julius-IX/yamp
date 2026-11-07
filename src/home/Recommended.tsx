import { useState } from "react";
import { COMMANDS } from "@/lib/commands";
import { readHistory } from "@/player/history";
import type { Track } from "@/lib/types";
import { interleave, mapLimit } from "@/browse/tracks";
import TrackShelf from "@/browse/TrackShelf";
import { useCached } from "@/browse/useCached";

const SEEDS = 3;
const PER_SEED = 10;
const MAX_TRACKS = 20;
const CONCURRENCY = 2;

/** Recently played first, then liked songs to fill the gap on a library that has barely been used. */
async function pickSeeds(recent: Track[]): Promise<Track[]> {
  if (recent.length >= SEEDS) return recent.slice(0, SEEDS);
  const liked = await COMMANDS.likes.list({ limit: SEEDS, offset: 0 });
  const seen = new Set(recent.map((t) => t.id));
  return [...recent, ...liked.filter((t) => !seen.has(t.id))].slice(0, SEEDS);
}

export default function Recommended() {
  // The history is read once. Playing a track would otherwise change the seeds and start new
  // provider queries while the tab sits in the background. Tapping Home again picks up the change.
  const [recent] = useState(() => readHistory().slice(0, SEEDS));
  const key = `recommended:${recent.map((t) => t.id).join(",")}`;

  const tracks = useCached(key, async () => {
    const seeds = await pickSeeds(recent);
    if (seeds.length === 0) return [];
    const lists = await mapLimit(seeds, CONCURRENCY, (s) =>
      COMMANDS.media.related({ id: s.id, limit: PER_SEED }).catch(() => []),
    );
    // what is already liked or was just played is not a recommendation
    const skip = new Set([...seeds.map((s) => s.id), ...(await COMMANDS.likes.ids())]);
    return interleave(lists, MAX_TRACKS, skip);
  }, []);

  return (
    <TrackShelf
      title="Recommended Songs"
      tracks={tracks}
      empty="Play or like a few songs to get recommendations."
    />
  );
}
