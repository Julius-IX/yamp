import { Box, Typography } from "@mui/material";
import { useLocation, useParams } from "react-router-dom";
import { COMMANDS } from "@/lib/commands";
import type { Item } from "@/lib/types";
import { ItemCard } from "@/browse/ItemShelf";
import { CardSkeleton } from "@/browse/MediaCard";
import { unique } from "@/browse/tracks";
import { useCached } from "@/browse/useCached";
import { eyebrow, GUTTER, heroTitle, subdued } from "@/themes/ui";
import { IS_MOBILE } from "@/util/platform";

const GHOSTS = 12;

const byline = (p: Item) => p.subtitle ?? "Playlist";

/** The playlists of one mood or genre. They wrap instead of scrolling, since there are many and all of them are worth a look. */
export default function MoodPage() {
  const { params = "" } = useParams<{ params: string }>();
  const title = (useLocation().state as { title?: string } | null)?.title;
  const playlists = useCached<Item[]>(`mood:${params}`, () => COMMANDS.media.moodPlaylists({ params }), []);

  return (
    <Box sx={{ pt: IS_MOBILE ? 10 : 9, pb: 2 }}>
      <Box sx={{ px: GUTTER, pb: 3 }}>
        <Typography sx={eyebrow}>PLAYLISTS</Typography>
        <Typography sx={heroTitle}>{title ?? "Mood"}</Typography>
      </Box>
      {playlists?.length === 0 && <Typography sx={{ ...subdued, px: GUTTER }}>No playlists could be loaded for this one.</Typography>}
      <Box sx={{ display: "flex", flexWrap: "wrap", gap: 1.5, px: GUTTER }}>
        {playlists
          ? unique(playlists).map((p) => <ItemCard key={p.id} item={p} byline={byline} />)
          : Array.from({ length: GHOSTS }, (_, i) => <CardSkeleton key={i} />)}
      </Box>
    </Box>
  );
}
