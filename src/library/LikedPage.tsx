import { useEffect, useState } from "react";
import { Box, Skeleton, Typography } from "@mui/material";
import TrackTable from "@/browse/TrackTable";
import { playtime } from "@/components/media/format";
import { useLikedCount } from "@/lib/likes";
import { logger } from "@/lib/log";
import type { Track } from "@/lib/types";
import { player } from "@/player/store";
import { GUTTER, radius, subdued } from "@/themes/ui";
import { IS_MOBILE } from "@/util/platform";
import Header, { COVER, PlayButtons } from "./Header";
import LikedArt from "./LikedArt";
import { loadLiked } from "./play";
import PlaylistSort, { sortLabel } from "./PlaylistSort";
import { useArranged } from "./useArranged";

const log = logger("library");

/**
 * The liked songs, newest first until they are rearranged, and then in that order with newly
 * liked songs on top. It is a playlist in how it looks and plays, but there is nothing to rename
 * or delete. A song leaves it by losing its heart, which reloads the list.
 */
export default function LikedPage() {
  const count = useLikedCount();
  const [loaded, setLoaded] = useState<Track[] | null>(null);
  const { shown: tracks, sort, setSort, sorted, move } = useArranged("liked", loaded);

  useEffect(() => {
    let stale = false;
    loadLiked()
      .then((t) => !stale && setLoaded(t))
      .catch((e) => {
        log.warn("couldn't load the liked songs:", e);
        if (!stale) setLoaded([]);
      });
    return () => {
      stale = true;
    };
  }, [count]);

  return (
    <Box sx={{ pt: IS_MOBILE ? 10 : 9, pb: 2 }}>
      <Header
        art={<LikedArt sx={{ width: COVER, height: COVER, borderRadius: radius.card, fontSize: COVER / 2.5, boxShadow: 8 }} />}
        label="PLAYLIST"
        title="Liked Songs"
        meta={tracks ? [`${tracks.length} songs`, playtime(tracks)].filter(Boolean).join(" · ") : undefined}
      >
        <PlayButtons tracks={tracks ?? []} />
        <PlaylistSort sort={sort} onChange={setSort} />
      </Header>

      {tracks === null && Array.from({ length: 8 }, (_, i) => <Skeleton key={i} variant="rounded" height={44} sx={{ mx: GUTTER, my: 0.75 }} />)}
      {tracks?.length === 0 && <Typography sx={{ ...subdued, px: GUTTER }}>Songs that get a heart show up here.</Typography>}
      {sorted && (
        <Typography variant="caption" sx={{ ...subdued, px: GUTTER, pb: 1, display: "block" }}>
          Sorted by {sortLabel(sort.key).toLowerCase()}. Rows can be moved again in the playlist order.
        </Typography>
      )}
      {tracks && tracks.length > 0 && <TrackTable tracks={tracks} onPlay={(i) => player.play(tracks, i)} onMove={move} />}
    </Box>
  );
}
