import { Box, ButtonBase, IconButton, Typography } from "@mui/material";
import PlaylistAddIcon from "@mui/icons-material/PlaylistAdd";
import Cover from "@/player/Cover";
import { useHistory } from "@/player/history";
import { player } from "@/player/store";
import { GUTTER, radius, surface } from "@/themes/ui";
import { IS_MOBILE } from "@/util/platform";
import { unique } from "@/browse/tracks";

const COUNT = 6;
const THUMB = IS_MOBILE ? 48 : 56;

/** The last few tracks as one-tap pills. Left out until something was played, there is nothing to pick yet. */
export default function QuickPicks() {
  const picks = unique(useHistory()).slice(0, COUNT);
  if (picks.length === 0) return null;

  return (
    <Box
      sx={{
        display: "grid", gap: 1.5, px: GUTTER, pb: 4,
        gridTemplateColumns: `repeat(auto-fill, minmax(${IS_MOBILE ? 150 : 260}px, 1fr))`,
      }}
    >
      {picks.map((t, i) => (
        <Box key={`${t.id}:${i}`} sx={{ position: "relative", "&:hover .pick-queue": { opacity: 1 } }}>
          <ButtonBase
            onClick={() => player.play(picks, i)}
            sx={{
              width: "100%", display: "flex", alignItems: "center", gap: 1.5, overflow: "hidden", textAlign: "left",
              borderRadius: radius.row, ...surface,
              transition: "background-color 0.2s", "&:hover": { bgcolor: "action.hover" },
            }}
          >
            <Cover src={t.thumbnail} px={160} sx={{ width: THUMB, height: THUMB, borderRadius: 0, fontSize: 24 }} />
            <Typography variant="body2" noWrap sx={{ flex: 1, minWidth: 0, pr: IS_MOBILE ? 1.5 : 5, fontWeight: 600 }}>
              {t.title}
            </Typography>
          </ButtonBase>
          {!IS_MOBILE && (
            <IconButton
              className="pick-queue" size="small" title="Add to queue" aria-label="Add to queue"
              onClick={() => player.enqueue([t], "end")}
              sx={{ position: "absolute", right: 8, top: "50%", transform: "translateY(-50%)", opacity: 0, transition: "opacity 0.2s" }}
            >
              <PlaylistAddIcon fontSize="small" />
            </IconButton>
          )}
        </Box>
      ))}
    </Box>
  );
}
