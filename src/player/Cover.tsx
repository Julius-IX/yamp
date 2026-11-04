import { Avatar, type SxProps, type Theme } from "@mui/material";
import MusicNoteIcon from "@mui/icons-material/MusicNote";

/**
 * Artwork with a music note behind it. The Avatar fallback is a person icon, which is wrong for
 * a track, and it also shows for a thumbnail that fails to load.
 */
export default function Cover({ src, sx }: { src?: string | null; sx?: SxProps<Theme> }) {
  return (
    <Avatar
      variant="rounded"
      src={src ?? undefined}
      sx={[{ bgcolor: "action.selected", color: "text.disabled" }, ...(Array.isArray(sx) ? sx : [sx])]}
    >
      <MusicNoteIcon fontSize="inherit" />
    </Avatar>
  );
}
