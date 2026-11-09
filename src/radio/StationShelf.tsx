import { player } from "@/player/store";
import MediaCard from "@/browse/MediaCard";
import Shelf from "@/browse/Shelf";
import { type Station, stationTrack } from "./station";
import StationLogo from "./StationLogo";

/** The tags and the country under a card, which say more about a station than its bitrate. */
const byline = (s: Station) => [s.tags.slice(0, 3).join(", "), s.country].filter(Boolean).join(" · ");

/** A row of station cards that scrolls sideways. `stations` is undefined while loading. */
export default function StationShelf({ title, stations, empty }: { title: string; stations: Station[] | undefined; empty: string }) {
  const tracks = stations?.map(stationTrack);
  const play = (i: number) => tracks && player.play(tracks, i);

  return (
    <Shelf title={title} count={stations?.length ?? null} empty={empty}>
      {stations?.map((s, i) => (
        <MediaCard
          key={s.id}
          art={<StationLogo src={s.favicon} inset="14%" sx={{ width: "100%", aspectRatio: "1", fontSize: 40 }} />}
          title={s.name}
          subtitle={byline(s)}
          onClick={() => play(i)}
          onPlay={() => play(i)}
        />
      ))}
    </Shelf>
  );
}
