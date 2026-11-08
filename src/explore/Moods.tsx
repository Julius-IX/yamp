import { Box, ButtonBase, Skeleton, Typography } from "@mui/material";
import { useNavigate } from "react-router-dom";
import { GUTTER, radius, sectionTitle, subdued, surface } from "@/themes/ui";
import { MoodPlaylists } from "./paths";
import { useExploreList } from "./useExplore";

const GHOSTS = 16;

export default function Moods() {
  const moods = useExploreList("moods");
  const navigate = useNavigate();

  return (
    <Box component="section" sx={{ pb: 3, minWidth: 0 }}>
      <Box sx={{ px: GUTTER, pb: 1.5 }}>
        <Typography sx={sectionTitle}>Moods & Genres</Typography>
        {moods?.length === 0 && <Typography variant="body2" sx={subdued}>Moods and genres are not available right now.</Typography>}
      </Box>
      <Box sx={{ display: "grid", gap: 1.5, mx: GUTTER, gridTemplateColumns: "repeat(auto-fill, minmax(150px, 1fr))" }}>
        {moods
          ? moods.map((m) => (
              <ButtonBase
                key={m.params}
                // the title rides along, so the screen has it before its own query is back
                onClick={() => navigate(MoodPlaylists.to({ params: m.params }), { state: { title: m.title } })}
                sx={{
                  ...surface, justifyContent: "flex-start", px: 2, py: 1.75, borderRadius: radius.row, textAlign: "left",
                  // the stripe is the color the provider gives the mood
                  borderLeft: 4, borderLeftColor: m.color ?? "primary.main",
                  "&:hover": { bgcolor: "action.hover" },
                }}
              >
                <Typography variant="body2" noWrap sx={{ fontWeight: 600 }}>{m.title}</Typography>
              </ButtonBase>
            ))
          : Array.from({ length: GHOSTS }, (_, i) => <Skeleton key={i} variant="rounded" height={52} />)}
      </Box>
    </Box>
  );
}
