import { COMMANDS } from "@/lib/commands";
import { useCountry, WORLDWIDE } from "@/lib/country";
import type { Item } from "@/lib/types";
import ItemShelf from "@/browse/ItemShelf";
import { useCached } from "@/browse/useCached";

export default function TopArtists() {
  const [country] = useCountry();
  // the country is part of the key, so choosing another one in settings loads its chart
  const artists = useCached<Item[]>(
    `top-artists:${country || WORLDWIDE}`,
    () => COMMANDS.media.topArtists({ country: country || null }),
    [],
  );
  const rank = new Map(artists?.map((a, i) => [a.id, i + 1]));

  return (
    <ItemShelf
      title="Top Artists"
      items={artists}
      empty="The artist chart is not available right now."
      byline={(a) => `#${rank.get(a.id)}`}
    />
  );
}
