import { useEffect, useState } from "react";
import { Box, Skeleton, Typography } from "@mui/material";
import TrackTable from "@/browse/TrackTable";
import { playtime } from "@/components/media/format";
import { useNote } from "@/lib/arrange";
import { useLocalVersion } from "@/lib/local";
import { logger } from "@/lib/log";
import type { Track } from "@/lib/types";
import { player } from "@/player/store";
import { GUTTER, radius, subdued } from "@/themes/ui";
import { IS_MOBILE } from "@/util/platform";
import Header, { COVER, PlayButtons } from "./Header";
import LocalArt from "./LocalArt";
import { loadLocal } from "./play";
import NoteButton from "./NoteButton";
import PlaylistSort, { sortLabel } from "./PlaylistSort";
import { useArranged } from "./useArranged";

const log = logger("library");

/**
 * The songs that are on this device, newest first until they are rearranged. Downloads and the
 * files of the folders from the data settings are listed together. Like the liked songs it is
 * not a playlist to rename or delete, a song leaves it when its download is removed or its
 * folder is.
 */
export default function LocalPage() {
  const version = useLocalVersion();
  const [loaded, setLoaded] = useState<Track[] | null>(null);
  const note = useNote("local");
  const { shown: tracks, sort, setSort, sorted, move } = useArranged("local", loaded);

  useEffect(() => {
    let stale = false;
    loadLocal()
      .then((t) => !stale && setLoaded(t))
      .catch((e) => {
        log.warn("couldn't load the local songs:", e);
        if (!stale) setLoaded([]);
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
        meta={tracks ? [note, `${tracks.length} songs`, playtime(tracks)].filter(Boolean).join(" · ") : undefined}
      >
        <PlayButtons tracks={tracks ?? []} />
        <PlaylistSort sort={sort} onChange={setSort} />
        <NoteButton slot="local" />
      </Header>

      {tracks === null && Array.from({ length: 8 }, (_, i) => <Skeleton key={i} variant="rounded" height={44} sx={{ mx: GUTTER, my: 0.75 }} />)}
      {tracks?.length === 0 && (
        <Typography sx={{ ...subdued, px: GUTTER }}>
          Songs that are downloaded, or found in the folders added in the data settings, show up here.
        </Typography>
      )}
      {sorted && (
        <Typography variant="caption" sx={{ ...subdued, px: GUTTER, pb: 1, display: "block" }}>
          Sorted by {sortLabel(sort.key).toLowerCase()}. Rows can be moved again in the playlist order.
        </Typography>
      )}
      {tracks && tracks.length > 0 && <TrackTable tracks={tracks} onPlay={(i) => player.play(tracks, i)} onMove={move} />}
    </Box>
  );
}
