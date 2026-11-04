import { Box, IconButton, Slider, Tooltip } from "@mui/material";
import VolumeDownIcon from "@mui/icons-material/VolumeDown";
import VolumeOffIcon from "@mui/icons-material/VolumeOff";
import VolumeUpIcon from "@mui/icons-material/VolumeUp";
import { player, usePlayer } from "./store";

/** Desktop only. A phone has its hardware keys for this. */
export default function VolumeControl() {
  const { volume, muted } = usePlayer();
  const silent = muted || volume === 0;
  const Icon = silent ? VolumeOffIcon : volume < 0.5 ? VolumeDownIcon : VolumeUpIcon;

  return (
    <Box sx={{ display: "flex", alignItems: "center", gap: 0.5, width: 150 }}>
      <Tooltip title={silent ? "Unmute" : "Mute"}>
        <IconButton size="small" aria-label={silent ? "Unmute" : "Mute"} onClick={player.toggleMute}>
          <Icon fontSize="small" />
        </IconButton>
      </Tooltip>
      <Slider
        size="small"
        aria-label="Volume"
        min={0}
        max={1}
        step={0.01}
        value={silent ? 0 : volume}
        // saved only when the drag ends, a move would otherwise write the settings file on every step
        onChange={(_, v) => player.setVolume(v, false)}
        onChangeCommitted={(_, v) => player.setVolume(v)}
        sx={{
          "& .MuiSlider-rail": { opacity: 0.25 },
          "& .MuiSlider-thumb": { width: 10, height: 10 },
        }}
      />
    </Box>
  );
}
