import { Box, ButtonBase, Skeleton, Typography } from "@mui/material";
import ArtistLink from "@/browse/ArtistLink";
import { unique } from "@/browse/tracks";
import Cover from "@/player/Cover";
import { player } from "@/player/store";
import { faint, GUTTER, radius, sectionTitle, subdued } from "@/themes/ui";
import { IS_MOBILE } from "@/util/platform";
import { useExploreList } from "./useExplore";

const GHOSTS = 8;
const COVER = 44;

/**
 * The trending songs as a numbered grid. A row reads left to right, so the numbers run across
 * and the first row holds the top of the chart. Playing a row queues the whole chart from it.
 */
export default function Trending() {
  const tracks = useExploreList("trending");
  const list = tracks && unique(tracks);

  return (
    <Box component="section" sx={{ pb: 3, minWidth: 0 }}>
      <Box sx={{ px: GUTTER, pb: 1.5 }}>
        <Typography sx={sectionTitle}>Trending</Typography>
        {list?.length === 0 && <Typography variant="body2" sx={subdued}>Nothing is trending right now.</Typography>}
      </Box>
      <Box
        sx={{
          display: "grid", gap: 0.5, mx: GUTTER,
          gridTemplateColumns: IS_MOBILE ? "minmax(0, 1fr)" : "repeat(auto-fill, minmax(300px, 1fr))",
        }}
      >
        {list
          ? list.map((t, i) => (
              <ButtonBase
                key={`${t.id}:${i}`}
                onClick={() => player.play(list, i)}
                sx={{
                  display: "flex", justifyContent: "flex-start", gap: 1.5, p: 0.75, borderRadius: radius.row,
                  textAlign: "left", minWidth: 0, "&:hover": { bgcolor: "action.hover" },
                }}
              >
                <Typography sx={{ width: 28, flexShrink: 0, textAlign: "center", fontWeight: 700, ...faint }}>{i + 1}</Typography>
                <Cover src={t.thumbnail} px={96} sx={{ width: COVER, height: COVER, fontSize: 20 }} />
                <Box sx={{ minWidth: 0 }}>
                  <Typography variant="body2" noWrap sx={{ fontWeight: 600 }}>{t.title}</Typography>
                  <Typography variant="caption" noWrap component="div" sx={subdued}>
                    <ArtistLink artist={t.artist} />
                  </Typography>
                </Box>
              </ButtonBase>
            ))
          : Array.from({ length: GHOSTS }, (_, i) => (
              <Box key={i} sx={{ display: "flex", alignItems: "center", gap: 1.5, p: 0.75 }}>
                <Box sx={{ width: 28 }} />
                <Skeleton variant="rounded" width={COVER} height={COVER} />
                <Box sx={{ flex: 1 }}>
                  <Skeleton width="70%" />
                  <Skeleton width="40%" />
                </Box>
              </Box>
            ))}
      </Box>
    </Box>
  );
}
