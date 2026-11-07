import { useEffect, useMemo, useState } from "react";
import { Box, type SxProps, type Theme } from "@mui/material";
import MusicNoteIcon from "@mui/icons-material/MusicNote";
import { artworkCandidates } from "@/lib/artwork";

/**
 * Artwork with a music note behind it, which stays visible when there is no image or none loads.
 * The Avatar fallback is a person icon, which is wrong for a track. `px` is the size to ask the
 * CDN for, pass a large one only where the cover is shown big.
 */
export default function Cover({ src, px = 256, sx }: { src?: string | null; px?: number; sx?: SxProps<Theme> }) {
  const candidates = useMemo(() => artworkCandidates(src, px), [src, px]);
  const [attempt, setAttempt] = useState(0);
  useEffect(() => setAttempt(0), [candidates]);
  const url = candidates[attempt];

  return (
    <Box
      sx={[
        {
          position: "relative", overflow: "hidden", flexShrink: 0, borderRadius: 1, display: "flex",
          alignItems: "center", justifyContent: "center", bgcolor: "action.selected", color: "text.disabled",
        },
        ...(Array.isArray(sx) ? sx : [sx]),
      ]}
    >
      <MusicNoteIcon fontSize="inherit" />
      {url && (
        <Box
          component="img"
          src={url}
          alt=""
          decoding="async"
          // only what is near the screen is fetched, a shelf holds dozens. No referrer, so a CDN
          // that refuses requests from other origins treats this like any other client
          loading="lazy"
          referrerPolicy="no-referrer"
          onError={() => setAttempt((a) => a + 1)}
          sx={{ position: "absolute", inset: 0, width: "100%", height: "100%", objectFit: "cover" }}
        />
      )}
    </Box>
  );
}
