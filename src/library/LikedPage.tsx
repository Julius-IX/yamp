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

const log = logger("library");

/**
 * The liked songs, newest first. It is a playlist in how it looks and plays, but there is nothing
 * to rename or delete. A song leaves it by losing its heart, which reloads the list.
 */
export default function LikedPage() {
  const count = useLikedCount();
  const [tracks, setTracks] = useState<Track[] | null>(null);

  useEffect(() => {
    let stale = false;
    loadLiked()
      .then((t) => !stale && setTracks(t))
      .catch((e) => {
        log.warn("couldn't load the liked songs:", e);
        if (!stale) setTracks([]);
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
      </Header>

      {tracks === null && Array.from({ length: 8 }, (_, i) => <Skeleton key={i} variant="rounded" height={44} sx={{ mx: GUTTER, my: 0.75 }} />)}
      {tracks?.length === 0 && <Typography sx={{ ...subdued, px: GUTTER }}>Songs that get a heart show up here.</Typography>}
      {tracks && tracks.length > 0 && <TrackTable tracks={tracks} onPlay={(i) => player.play(tracks, i)} />}
    </Box>
  );
}
