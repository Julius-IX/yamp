import type { ReactNode } from "react";
import { player } from "@/player/store";
import type { Track } from "@/lib/types";
import { video } from "@/video/store";
import ArtistLink from "./ArtistLink";
import type { CardVariant } from "./MediaCard";
import MediaCard from "./MediaCard";
import Shelf from "./Shelf";
import { unique } from "./tracks";

/** A shelf of playable tracks. `tracks` is undefined or null while loading. Playing a card queues the whole row from it. */
export default function TrackShelf({
  title, tracks, empty, variant = "square", subtitle = (t) => <ArtistLink artist={t.artist} />, hideWhenEmpty, videos,
}: {
  title: string;
  tracks: Track[] | null | undefined;
  empty: string;
  variant?: CardVariant;
  subtitle?: (t: Track) => ReactNode;
  hideWhenEmpty?: boolean;
  /** The cards are videos. They open in the video modal and do not touch the queue. */
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
          onClick={() => (videos ? video.open(t) : player.play(list, i))}
          onQueue={videos ? undefined : () => player.enqueue([t], "end")}
        />
      ))}
    </Shelf>
  );
}
