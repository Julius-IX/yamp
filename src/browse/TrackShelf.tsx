import { player } from "@/player/store";
import type { Track } from "@/lib/types";
import type { CardVariant } from "./MediaCard";
import MediaCard from "./MediaCard";
import Shelf from "./Shelf";
import { unique } from "./tracks";

/** A shelf of playable tracks. `tracks` is undefined or null while loading. Playing a card queues the whole row from it. */
export default function TrackShelf({
  title, tracks, empty, variant = "square", subtitle = (t) => t.artist, hideWhenEmpty,
}: {
  title: string;
  tracks: Track[] | null | undefined;
  empty: string;
  variant?: CardVariant;
  subtitle?: (t: Track) => string;
  hideWhenEmpty?: boolean;
}) {
  const list = tracks && unique(tracks);
  return (
    <Shelf title={title} count={list?.length ?? null} empty={empty} variant={variant} hideWhenEmpty={hideWhenEmpty}>
      {list?.map((t, i) => (
        <MediaCard
          key={`${t.id}:${i}`}
          thumbnail={t.thumbnail}
          title={t.title}
          subtitle={subtitle(t)}
          variant={variant}
          onClick={() => player.play(list, i)}
          onQueue={() => player.enqueue([t], "end")}
        />
      ))}
    </Shelf>
  );
}
