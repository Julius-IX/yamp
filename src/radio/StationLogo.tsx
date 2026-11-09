import { useEffect, useMemo, useState } from "react";
import { Box, type SxProps, type Theme } from "@mui/material";
import SensorsIcon from "@mui/icons-material/Sensors";

/** A station that lists its logo over HTTP is tried over HTTPS first, since many serve both. */
const candidates = (src: string | null | undefined) =>
  !src ? [] : src.startsWith("http:") ? [`https:${src.slice(5)}`, src] : [src];

/**
 * The logo of a station. Logos come in every shape, so the image is fitted inside the tile and
 * not cropped, and `inset` keeps it off the edges. The radio icon behind it stays visible when
 * there is no logo or none loads.
 */
export default function StationLogo({ src, inset = "0", sx }: { src?: string | null; inset?: string; sx?: SxProps<Theme> }) {
  const urls = useMemo(() => candidates(src), [src]);
  const [attempt, setAttempt] = useState(0);
  useEffect(() => setAttempt(0), [urls]);
  const url = urls[attempt];

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
      <SensorsIcon fontSize="inherit" />
      {url && (
        <Box
          component="img"
          src={url}
          alt=""
          decoding="async"
          loading="lazy"
          referrerPolicy="no-referrer"
          onError={() => setAttempt((a) => a + 1)}
          sx={{ position: "absolute", inset: 0, width: "100%", height: "100%", objectFit: "contain", boxSizing: "border-box", padding: inset }}
        />
      )}
    </Box>
  );
}
