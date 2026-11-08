import { type ChangeEvent, useRef, useState } from "react";
import { Box, CircularProgress, IconButton, Tooltip } from "@mui/material";
import PhotoCameraIcon from "@mui/icons-material/PhotoCamera";
import { coverOf, setPlaylistCover } from "@/lib/library";
import type { LibraryPlaylist } from "@/lib/types";
import Cover from "@/player/Cover";
import { radius } from "@/themes/ui";
import { IS_MOBILE } from "@/util/platform";
import { COVER } from "./Header";

/** The cover of a playlist with a button that opens a picker for another image. */
export default function CoverPicker({ playlist }: { playlist: LibraryPlaylist | undefined }) {
  const input = useRef<HTMLInputElement>(null);
  const [busy, setBusy] = useState(false);

  const pick = async (e: ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    // cleared so that picking the same file again still fires a change
    e.target.value = "";
    if (!file || !playlist) return;
    setBusy(true);
    await setPlaylistCover(playlist.id, file);
    setBusy(false);
  };

  return (
    <Box sx={{ position: "relative", flexShrink: 0, "&:hover .cover-edit": { opacity: 1 } }}>
      <Cover
        src={playlist && coverOf(playlist)}
        px={640}
        sx={{ width: COVER, height: COVER, borderRadius: radius.card, fontSize: 64, boxShadow: 8 }}
      />
      <input ref={input} type="file" accept="image/*" hidden onChange={(e) => void pick(e)} />
      <Tooltip title="Change cover">
        <span>
          <IconButton
            className="cover-edit"
            aria-label="Change cover"
            disabled={!playlist || busy}
            onClick={() => input.current?.click()}
            sx={{
              position: "absolute", right: 8, bottom: 8, bgcolor: "rgba(0,0,0,0.6)", color: "#fff",
              opacity: IS_MOBILE || busy ? 1 : 0, transition: "opacity 0.2s",
              "&:hover": { bgcolor: "rgba(0,0,0,0.8)" },
            }}
          >
            {busy ? <CircularProgress size={20} color="inherit" /> : <PhotoCameraIcon fontSize="small" />}
          </IconButton>
        </span>
      </Tooltip>
    </Box>
  );
}
