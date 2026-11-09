import type { SxProps, Theme } from "@mui/material";
import FavoriteIcon from "@mui/icons-material/Favorite";
import ThemedArt from "./ThemedArt";

/** The cover of the liked songs. */
export default function LikedArt({ sx }: { sx?: SxProps<Theme> }) {
  return <ThemedArt icon={<FavoriteIcon fontSize="inherit" />} sx={sx} />;
}
