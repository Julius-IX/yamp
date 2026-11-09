import { useEffect, useState } from "react";
import { Box, Skeleton, Typography } from "@mui/material";
import TrackTable from "@/browse/TrackTable";
import { useLocalVersion } from "@/lib/local";
import { logger } from "@/lib/log";
import type { Track } from "@/lib/types";
import { player } from "@/player/store";
import { GUTTER, radius, subdued } from "@/themes/ui";
import { IS_MOBILE } from "@/util/platform";
import Header, { COVER, PlayButtons } from "./Header";
import LocalArt from "./LocalArt";
import { loadLocal } from "./play";

const log = logger("library");

/**
 * The songs that are on this device, newest first. Downloads and the files of the folders from
 * the data settings are listed together. Like the liked songs it is not a playlist to edit, a
 * song leaves it when its download is removed or its folder is.
 */
export default function LocalPage() {
  const version = useLocalVersion();
  const [tracks, setTracks] = useState<Track[] | null>(null);

  useEffect(() => {
    let stale = false;
    loadLocal()
      .then((t) => !stale && setTracks(t))
      .catch((e) => {
        log.warn("couldn't load the local songs:", e);
        if (!stale) setTracks([]);
      });
    return () => {
      stale = true;
    };
  }, [version]);

  return (
    <Box sx={{ pt: IS_MOBILE ? 10 : 9, pb: 2 }}>
      <Header
        art={<LocalArt sx={{ width: COVER, height: COVER, borderRadius: radius.card, fontSize: COVER / 2.5, boxShadow: 8 }} />}
        label="PLAYLIST"
        title="Local"
        meta={tracks ? `${tracks.length} songs` : undefined}
      >
        <PlayButtons tracks={tracks ?? []} />
      </Header>

      {tracks === null && Array.from({ length: 8 }, (_, i) => <Skeleton key={i} variant="rounded" height={44} sx={{ mx: GUTTER, my: 0.75 }} />)}
      {tracks?.length === 0 && (
        <Typography sx={{ ...subdued, px: GUTTER }}>
          Songs that are downloaded, or found in the folders added in the data settings, show up here.
        </Typography>
      )}
      {tracks && tracks.length > 0 && <TrackTable tracks={tracks} onPlay={(i) => player.play(tracks, i)} />}
    </Box>
  );
}
