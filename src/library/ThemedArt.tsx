import type { ReactNode } from "react";
import { Box, type SxProps, type Theme } from "@mui/material";

/**
 * The cover of a playlist that the app makes itself, an icon on the accent. It is drawn from the
 * theme and not fetched, so it always matches the accent and there is no image that could be
 * swapped, which these playlists do not allow.
 */
export default function ThemedArt({ icon, sx }: { icon: ReactNode; sx?: SxProps<Theme> }) {
  return (
    <Box
      sx={[
        {
          display: "flex", alignItems: "center", justifyContent: "center", flexShrink: 0, borderRadius: 1,
          color: (t: Theme) => t.palette.getContrastText(t.palette.primary.main),
          background: (t: Theme) => `linear-gradient(135deg, ${t.palette.primary.dark}, ${t.palette.primary.light})`,
        },
        ...(Array.isArray(sx) ? sx : [sx]),
      ]}
    >
      {icon}
    </Box>
  );
}
