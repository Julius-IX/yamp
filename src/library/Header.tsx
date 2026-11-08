import type { ReactNode } from "react";
import { Box, IconButton, Skeleton, Tooltip, Typography } from "@mui/material";
import PlayArrowIcon from "@mui/icons-material/PlayArrow";
import ShuffleIcon from "@mui/icons-material/Shuffle";
import type { Track } from "@/lib/types";
import { player } from "@/player/store";
import { accentFill, eyebrow, GUTTER, heroTitle, subdued } from "@/themes/ui";
import { IS_MOBILE } from "@/util/platform";
import { shuffled } from "./play";

/** The size of the artwork in the header, for the screen to size its own. */
export const COVER = IS_MOBILE ? 140 : 200;

/**
 * The top of a screen for one list of tracks, with the artwork, a title and the actions below.
 * The title is a skeleton while it is undefined, for a list whose name is not known yet.
 */
export default function Header({
  art, label, title, meta, children,
}: { art: ReactNode; label: string; title: string | undefined; meta?: string; children?: ReactNode }) {
  return (
    <>
      <Box sx={{ display: "flex", flexDirection: IS_MOBILE ? "column" : "row", alignItems: IS_MOBILE ? "flex-start" : "flex-end", gap: 3, px: GUTTER, pb: 3 }}>
        {art}
        <Box sx={{ minWidth: 0 }}>
          <Typography sx={eyebrow}>{label}</Typography>
          {title !== undefined ? <Typography sx={heroTitle}>{title}</Typography> : <Skeleton width={320} height={56} />}
          <Typography sx={{ ...subdued, mt: 0.5 }}>{meta || "\u00A0"}</Typography>
        </Box>
      </Box>
      <Box sx={{ display: "flex", alignItems: "center", gap: 1.5, px: GUTTER, pb: 2 }}>{children}</Box>
    </>
  );
}

export function PlayButtons({ tracks }: { tracks: Track[] }) {
  const empty = tracks.length === 0;
  return (
    <>
      <IconButton aria-label="Play" disabled={empty} onClick={() => player.play(tracks, 0)} sx={{ width: 52, height: 52, ...accentFill }}>
        <PlayArrowIcon />
      </IconButton>
      <Tooltip title="Shuffle">
        <span>
          <IconButton aria-label="Shuffle" disabled={empty} onClick={() => player.play(shuffled(tracks), 0)}>
            <ShuffleIcon />
          </IconButton>
        </span>
      </Tooltip>
    </>
  );
}
