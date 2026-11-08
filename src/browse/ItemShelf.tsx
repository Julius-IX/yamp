import type { ReactNode } from "react";
import { useNavigate } from "react-router-dom";
import { COMMANDS } from "@/lib/commands";
import { logger } from "@/lib/log";
import type { Item } from "@/lib/types";
import { player } from "@/player/store";
import { ArtistLine } from "./ArtistLink";
import MediaCard from "./MediaCard";
import { Album, Artist, Playlist } from "./paths";
import { unique } from "./tracks";
import Shelf from "./Shelf";
import { useCached } from "./useCached";

const log = logger("browse");

const pathOf = (item: Item) =>
  item.kind === "artist" ? Artist.to({ id: item.id }) : item.kind === "album" ? Album.to({ id: item.id }) : Playlist.to({ id: item.id });

const defaultByline = (item: Item): ReactNode =>
  item.kind === "artist"
    ? (item.subtitle ?? "Artist")
    : item.kind === "album"
      ? <ArtistLine artist={item.subtitle} rest={[item.releaseYear]} />
      : (item.subtitle ?? item.label ?? "Playlist");

async function playItem(item: Item) {
  try {
    const { tracks } = item.kind === "album" ? await COMMANDS.media.album({ id: item.id }) : await COMMANDS.media.playlist({ id: item.id });
    if (tracks.length > 0) player.play(tracks, 0);
  } catch (e) {
    log.warn(`couldn't play "${item.title}":`, e);
  }
}

/** One card of a listing item. A shelf lays them out in a row and a screen can lay them out any other way. */
export function ItemCard({ item, byline, thumbnail }: { item: Item; byline: (item: Item) => ReactNode; thumbnail?: string | null }) {
  const navigate = useNavigate();
  return (
    <MediaCard
      thumbnail={thumbnail ?? item.thumbnail}
      title={item.title}
      subtitle={byline(item)}
      variant={item.kind === "artist" ? "round" : "square"}
      // the listing item rides along, so the screen has a title and a cover before its own query is back
      onClick={() => navigate(pathOf(item), { state: { item } })}
      onPlay={item.kind === "artist" ? undefined : () => void playItem(item)}
    />
  );
}

/**
 * Related artists come without a picture. The artist page has one, and opening it later is
 * likely, so it is fetched under the key that page reads. The query is shared, not wasted.
 */
function ArtistCard({ item, byline }: { item: Item; byline: (item: Item) => ReactNode }) {
  const page = useCached(`artist:${item.id}`, () => COMMANDS.media.artist({ id: item.id }), null);
  return <ItemCard item={item} byline={byline} thumbnail={page?.thumbnail} />;
}

/** A shelf of artists, albums or playlists. `items` is undefined while loading. */
export default function ItemShelf({
  title, items, empty, hideWhenEmpty, controls, byline = defaultByline,
}: {
  title: string;
  items: Item[] | undefined;
  empty: string;
  hideWhenEmpty?: boolean;
  controls?: ReactNode;
  /** The line under a title. The default suits a search result. */
  byline?: (item: Item) => ReactNode;
}) {
  const list = items && unique(items);
  const round = list?.[0]?.kind === "artist";
  return (
    <Shelf
      title={title}
      count={list?.length ?? null}
      empty={empty}
      variant={round ? "round" : "square"}
      hideWhenEmpty={hideWhenEmpty}
      controls={controls}
    >
      {list?.map((item, i) =>
        item.kind === "artist" && !item.thumbnail ? (
          <ArtistCard key={`${item.id}:${i}`} item={item} byline={byline} />
        ) : (
          <ItemCard key={`${item.id}:${i}`} item={item} byline={byline} />
        ),
      )}
    </Shelf>
  );
}
