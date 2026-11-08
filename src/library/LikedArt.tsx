import { Box, type SxProps, type Theme } from "@mui/material";
import FavoriteIcon from "@mui/icons-material/Favorite";

/**
 * The cover of the liked songs. It is drawn from the theme and not fetched, so it always matches
 * the accent and there is no image that could be swapped, which the liked songs do not allow.
 */
export default function LikedArt({ sx }: { sx?: SxProps<Theme> }) {
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
      <FavoriteIcon fontSize="inherit" />
    </Box>
  );
}
