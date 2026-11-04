import { Box, CircularProgress, IconButton, Tooltip } from "@mui/material";
import PauseIcon from "@mui/icons-material/Pause";
import PlayArrowIcon from "@mui/icons-material/PlayArrow";
import RepeatIcon from "@mui/icons-material/Repeat";
import RepeatOneIcon from "@mui/icons-material/RepeatOne";
import ShuffleIcon from "@mui/icons-material/Shuffle";
import SkipNextIcon from "@mui/icons-material/SkipNext";
import SkipPreviousIcon from "@mui/icons-material/SkipPrevious";
import { player, usePlayer } from "./store";

/** Shuffle, previous, play or pause, next and repeat. `large` is the touch size of the full screen player. */
export default function TransportControls({ large = false }: { large?: boolean }) {
  const { playing, loading, shuffle, repeat } = usePlayer();
  const side = large ? "large" : "medium";
  const main = large ? 64 : 40;
  const RepeatMode = repeat === "one" ? RepeatOneIcon : RepeatIcon;

  return (
    <Box sx={{ display: "flex", alignItems: "center", justifyContent: "center", gap: large ? 1.5 : 0.5 }}>
      <Tooltip title={shuffle ? "Shuffle on" : "Shuffle off"}>
        <IconButton size={side} aria-label="Shuffle" color={shuffle ? "primary" : "default"} onClick={player.toggleShuffle}>
          <ShuffleIcon fontSize="inherit" />
        </IconButton>
      </Tooltip>
      <Tooltip title="Previous">
        <IconButton size={side} aria-label="Previous" onClick={player.prev}>
          <SkipPreviousIcon fontSize="inherit" />
        </IconButton>
      </Tooltip>
      <Tooltip title={playing ? "Pause" : "Play"}>
        <IconButton
          aria-label={playing ? "Pause" : "Play"}
          onClick={player.togglePlay}
          sx={{
            position: "relative", width: main, height: main, fontSize: main * 0.6,
            bgcolor: "text.primary", color: "background.default",
            "&:hover": { bgcolor: "text.secondary" },
          }}
        >
          {playing ? <PauseIcon fontSize="inherit" /> : <PlayArrowIcon fontSize="inherit" />}
          {loading && (
            <CircularProgress
              size={main - 4}
              thickness={3}
              sx={{ position: "absolute", inset: 2, color: "primary.main" }}
            />
          )}
        </IconButton>
      </Tooltip>
      <Tooltip title="Next">
        <IconButton size={side} aria-label="Next" onClick={player.next}>
          <SkipNextIcon fontSize="inherit" />
        </IconButton>
      </Tooltip>
      <Tooltip title={`Repeat ${repeat}`}>
        <IconButton size={side} aria-label="Repeat" color={repeat === "off" ? "default" : "primary"} onClick={player.cycleRepeat}>
          <RepeatMode fontSize="inherit" />
        </IconButton>
      </Tooltip>
    </Box>
  );
}
