import type { ReactNode } from "react";
import { player } from "@/player/store";
import type { Track } from "@/lib/types";
import { video } from "@/video/store";
import ArtistLink from "./ArtistLink";
import type { CardVariant } from "./MediaCard";
import MediaCard from "./MediaCard";
import Shelf from "./Shelf";
import { unique } from "./tracks";

/**
 * A shelf of playable tracks. `tracks` is undefined or null while loading. Playing a card queues
 * the whole row from it.
 */
export default function TrackShelf({
  title, tracks, empty, variant = "square", subtitle = (t) => <ArtistLink artist={t.artist} />, hideWhenEmpty, videos,
}: {
  title: string;
  tracks: Track[] | null | undefined;
  empty: string;
  variant?: CardVariant;
  subtitle?: (t: Track) => ReactNode;
  hideWhenEmpty?: boolean;
  /**
   * The cards are videos. They are tracks like any other and play from the whole row too, but a
   * click on the card also opens the picture. The play button on the card plays without it.
   */
  videos?: boolean;
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
          onClick={() => (videos ? video.watch(list, i) : player.play(list, i))}
          onPlay={videos ? () => player.play(list, i) : undefined}
          onQueue={() => player.enqueue([t], "end")}
        />
      ))}
    </Shelf>
  );
}
