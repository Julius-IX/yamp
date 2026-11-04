import { useState } from "react";
import { Box, Slider, Typography } from "@mui/material";
import { duration as clock } from "@/components/media/format";
import { player, usePlayer, usePlayerTime } from "./store";

const time = { minWidth: 36, fontVariantNumeric: "tabular-nums" } as const;

/** Elapsed time, a slider and the length. Nothing is sent to the audio element until the drag ends. */
export default function SeekBar() {
  const { index } = usePlayer();
  const { position, duration } = usePlayerTime();
  const [drag, setDrag] = useState<number | null>(null);
  const shown = Math.min(drag ?? position, duration || 0);

  return (
    <Box sx={{ display: "flex", alignItems: "center", gap: 1, width: "100%" }}>
      <Typography variant="caption" color="text.secondary" sx={{ ...time, textAlign: "right" }}>
        {clock(index < 0 ? 0 : shown)}
      </Typography>
      <Slider
        size="small"
        aria-label="Seek"
        min={0}
        max={duration || 1}
        step={1}
        value={shown}
        disabled={duration <= 0}
        onChange={(_, v) => setDrag(v)}
        onChangeCommitted={(_, v) => {
          player.seek(v);
          setDrag(null);
        }}
        sx={{
          py: 1.25,
          "& .MuiSlider-rail": { opacity: 0.25 },
          // the knob only shows on hover or while dragged, so the bar stays thin at rest
          "& .MuiSlider-thumb": { width: 10, height: 10, opacity: 0, transition: "opacity 0.15s" },
          "&:hover .MuiSlider-thumb, & .MuiSlider-thumb.Mui-active, & .MuiSlider-thumb.Mui-focusVisible": { opacity: 1 },
          "@media (hover: none)": { "& .MuiSlider-thumb": { opacity: 1 } },
        }}
      />
      <Typography variant="caption" color="text.secondary" sx={time}>
        {duration > 0 ? clock(duration) : "-:--"}
      </Typography>
    </Box>
  );
}
