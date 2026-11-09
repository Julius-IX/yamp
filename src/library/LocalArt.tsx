import type { SxProps, Theme } from "@mui/material";
import FolderIcon from "@mui/icons-material/Folder";
import ThemedArt from "./ThemedArt";

/** The cover of the local songs. */
export default function LocalArt({ sx }: { sx?: SxProps<Theme> }) {
  return <ThemedArt icon={<FolderIcon fontSize="inherit" />} sx={sx} />;
}
