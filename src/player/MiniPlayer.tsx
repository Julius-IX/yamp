import { useState } from "react";
import { Box, ButtonBase, IconButton, LinearProgress, Paper, Typography } from "@mui/material";
import PauseIcon from "@mui/icons-material/Pause";
import PlayArrowIcon from "@mui/icons-material/PlayArrow";
import SkipNextIcon from "@mui/icons-material/SkipNext";
import SkipPreviousIcon from "@mui/icons-material/SkipPrevious";
import AddToPlaylist from "@/library/AddToPlaylist";
import Cover from "./Cover";
import LikeButton from "./LikeButton";
import NowPlayingSheet from "./NowPlayingSheet";
import { sheen } from "@/themes/ui";
import { currentTrack, player, usePlayer, usePlayerTime } from "./store";

/** Progress as a hairline on the top edge, since the bar itself has no room for a seek slider. */
function Progress() {
  const { position, duration } = usePlayerTime();
  return (
    <LinearProgress
      variant="determinate"
      value={duration > 0 ? Math.min(100, (position / duration) * 100) : 0}
      // the default transition lags behind the four updates a second
      sx={{
        height: 2,
        "& .MuiLinearProgress-bar": {
          transition: "none",
          backgroundImage: (t) => `linear-gradient(90deg, ${t.palette.primary.dark}, ${t.palette.primary.light})`,
        },
      }}
    />
  );
}

/** Mobile only. Sits above the bottom navigation and is left out until something is queued. */
export default function MiniPlayer() {
  const state = usePlayer();
  const track = currentTrack(state);
  const [open, setOpen] = useState(false);

  if (!track) return null;
  return (
    <>
      <Paper square elevation={0} sx={{ flexShrink: 0, borderTop: 1, borderColor: "divider", ...sheen }}>
        <Progress />
        <Box sx={{ display: "flex", alignItems: "center", gap: 0.5, pl: 1.5, pr: 0.5, py: 0.75 }}>
          <ButtonBase
            onClick={() => setOpen(true)}
            aria-label="Open the player"
            sx={{ display: "flex", alignItems: "center", gap: 1.5, flex: 1, minWidth: 0, textAlign: "left", justifyContent: "flex-start" }}
          >
            <Cover src={track.thumbnail} sx={{ width: 44, height: 44, fontSize: 24 }} />
            <Box sx={{ minWidth: 0 }}>
              <Typography noWrap sx={{ fontWeight: 600 }}>{track.title}</Typography>
              <Typography variant="caption" color="text.secondary" noWrap component="div">{track.artist}</Typography>
            </Box>
          </ButtonBase>
          <LikeButton track={track} />
          {/* small, so the title keeps some room next to the five buttons */}
          <AddToPlaylist track={track} />
          <IconButton aria-label="Previous" onClick={player.prev}>
            <SkipPreviousIcon />
          </IconButton>
          <IconButton aria-label={state.playing ? "Pause" : "Play"} onClick={player.togglePlay}>
            {state.playing ? <PauseIcon /> : <PlayArrowIcon />}
          </IconButton>
          <IconButton aria-label="Next" onClick={player.next}>
            <SkipNextIcon />
          </IconButton>
        </Box>
      </Paper>
      <NowPlayingSheet open={open} onClose={() => setOpen(false)} />
    </>
  );
}
