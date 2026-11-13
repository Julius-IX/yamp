import { useMemo } from "react";
import { Box, Skeleton, Typography } from "@mui/material";
import SearchIcon from "@mui/icons-material/Search";
import { useSearchParams } from "react-router-dom";
import TrackTable from "@/browse/TrackTable";
import type { Track } from "@/lib/types";
import { player } from "@/player/store";
import { GUTTER, sectionTitle, subdued } from "@/themes/ui";
import { useLibrarySongs } from "./useLibrarySongs";

/**
 * Songs that contain every word of the query in their title, artist or album. A song whose
 * title holds the whole query comes first, the rest keeps the order of the library.
 */
function matching(songs: Track[], query: string): Track[] {
  const q = query.toLowerCase();
  const words = q.split(/\s+/).filter(Boolean);
  const hits = songs.filter((t) => {
    const text = `${t.title} ${t.artist} ${t.album ?? ""}`.toLowerCase();
    return words.every((w) => text.includes(w));
  });
  const named = (t: Track) => t.title.toLowerCase().includes(q);
  return [...hits.filter(named), ...hits.filter((t) => !named(t))];
}

/** The results screen of the library tab. It searches the songs saved here and never asks the provider. */
export default function LibrarySearchResults() {
  const query = (useSearchParams()[0].get("q") ?? "").trim();
  const songs = useLibrarySongs();
  const found = useMemo(() => (songs && query ? matching(songs, query) : undefined), [songs, query]);

  if (!query) {
    return (
      <Box sx={{ pt: 20, textAlign: "center", ...subdued }}>
        <SearchIcon sx={{ fontSize: 64 }} />
        <Typography>Search the songs in your library</Typography>
      </Box>
    );
  }
  if (found?.length === 0) {
    return <Typography sx={{ ...subdued, px: GUTTER, pt: 12 }}>No songs in your library match “{query}”.</Typography>;
  }

  return (
    <Box sx={{ pt: 10, pb: 2 }}>
      <Typography sx={{ ...sectionTitle, px: GUTTER, pb: 1.5 }}>Songs</Typography>
      {found ? (
        <TrackTable tracks={found} onPlay={(i) => player.play(found, i)} />
      ) : (
        Array.from({ length: 6 }, (_, i) => <Skeleton key={i} variant="rounded" height={44} sx={{ mx: GUTTER, my: 0.75 }} />)
      )}
    </Box>
  );
}
