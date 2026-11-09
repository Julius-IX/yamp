import { Box, ButtonBase, Skeleton, Typography } from "@mui/material";
import { useNavigate } from "react-router-dom";
import { faint, GUTTER, radius, surface } from "@/themes/ui";
import { IS_MOBILE } from "@/util/platform";
import { RadioTag } from "./paths";
import type { Tag } from "./api";

const GHOSTS = 16;

// The stripes only tell neighbors apart, so they are not taken from the theme.
const STRIPES = ["#e5533d", "#f0883e", "#e3b341", "#3fb950", "#2bb5a0", "#388bfd", "#a371f7", "#db61a2"];

/** The tags as tiles with their number of stations. `tags` is undefined while loading. */
export default function TagGrid({ tags }: { tags: Tag[] | undefined }) {
  const navigate = useNavigate();
  const min = IS_MOBILE ? 140 : 170;

  return (
    <Box sx={{ display: "grid", gap: 1.5, mx: GUTTER, gridTemplateColumns: `repeat(auto-fill, minmax(${min}px, 1fr))` }}>
      {tags
        ? tags.map((t, i) => (
            <ButtonBase
              key={t.name}
              onClick={() => navigate(RadioTag.to({ name: t.name }))}
              sx={{
                ...surface, justifyContent: "flex-start", alignItems: "baseline", gap: 0.75, px: 2, py: 1.75,
                borderRadius: radius.row, textAlign: "left", minWidth: 0,
                borderLeft: 4, borderLeftColor: STRIPES[i % STRIPES.length],
                "&:hover": { bgcolor: "action.hover" },
              }}
            >
              <Typography variant="body2" noWrap sx={{ fontWeight: 600 }}>{t.name}</Typography>
              <Typography variant="caption" sx={{ flexShrink: 0, ...faint }}>{t.count}</Typography>
            </ButtonBase>
          ))
        : Array.from({ length: GHOSTS }, (_, i) => <Skeleton key={i} variant="rounded" height={52} />)}
    </Box>
  );
}
