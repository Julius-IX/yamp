import { COMMANDS } from "@/lib/commands";
import type { Item } from "@/lib/types";
import ItemShelf from "@/browse/ItemShelf";
import { interleave, mapLimit } from "@/browse/tracks";
import { useCached } from "@/browse/useCached";

/** Each artist is a provider query, so only the first few followed artists are checked. */
const MAX_ARTISTS = 8;
const PER_ARTIST = 3;
const MAX_ITEMS = 20;
const CONCURRENCY = 2;

const byline = (i: Item) => [i.subtitle, i.releaseYear, i.label].filter(Boolean).join(" · ");

/** This year and last year, a release with no year cannot be told from an old one and is left out. */
async function load(): Promise<Item[]> {
  const artists = (await COMMANDS.artists.list()).slice(0, MAX_ARTISTS);
  const since = new Date().getFullYear() - 1;
  const lists = await mapLimit(artists, CONCURRENCY, async (a) => {
    const page = await COMMANDS.media.artist({ id: a.id }).catch(() => null);
    return [...(page?.albums ?? []), ...(page?.singles ?? [])]
      .filter((r) => (r.releaseYear ?? 0) >= since)
      .sort((x, y) => (y.releaseYear ?? 0) - (x.releaseYear ?? 0))
      .slice(0, PER_ARTIST);
  });
  return interleave(lists, MAX_ITEMS);
}

export default function FollowedReleases() {
  const items = useCached("followed-releases", load, []);
  return (
    <ItemShelf
      title="New From Artists You Follow"
      items={items}
      empty="New music from followed artists shows up here."
      byline={byline}
    />
  );
}
