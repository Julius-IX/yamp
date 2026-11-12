import { useState } from "react";
import { Box, Checkbox, IconButton, Tooltip, Typography } from "@mui/material";
import CloseIcon from "@mui/icons-material/Close";
import DownloadOutlinedIcon from "@mui/icons-material/DownloadOutlined";
import FileDownloadOffOutlinedIcon from "@mui/icons-material/FileDownloadOffOutlined";
import LibraryAddIcon from "@mui/icons-material/LibraryAdd";
import PlaylistAddIcon from "@mui/icons-material/PlaylistAdd";
import { PlaylistMenu } from "@/library/AddToPlaylist";
import { RemoveDownloadsDialog } from "@/library/DownloadButton";
import { downloadedAmong, downloadTrack, useDownloadedCount } from "@/lib/local";
import type { Track } from "@/lib/types";
import { player } from "@/player/store";
import { subdued } from "@/themes/ui";

/**
 * The bar above a table while it is picking tracks. The checkbox picks all of them or none, so a
 * whole playlist is one tap away from being downloaded. `picked` is what the actions work on.
 * Removing downloads only touches the picked songs that are downloaded.
 */
export default function SelectionBar({ picked, total, onToggleAll, onDone }: {
  picked: Track[];
  total: number;
  onToggleAll: () => void;
  onDone: () => void;
}) {
  const [playlists, setPlaylists] = useState<HTMLElement | null>(null);
  // the ids to remove are read when the button is pressed, so the question keeps the ones it was asked for
  const [removing, setRemoving] = useState<string[] | null>(null);
  const none = picked.length === 0;
  const downloaded = useDownloadedCount(picked.map((t) => t.id));

  const download = async () => {
    onDone();
    // one after the other, a long playlist would otherwise ask for every download at the same moment
    for (const track of picked) await downloadTrack(track);
  };

  return (
    <Box
      sx={{
        display: "flex", alignItems: "center", gap: 0.5, position: "sticky", top: 0, zIndex: 2,
        bgcolor: "background.default", borderBottom: 1, borderColor: "divider", pb: 0.5, mb: 0.5,
      }}
    >
      <Checkbox
        size="small"
        aria-label="Select all"
        checked={total > 0 && picked.length === total}
        indeterminate={!none && picked.length < total}
        onChange={onToggleAll}
      />
      <Typography variant="body2" sx={{ ...subdued, flex: 1 }}>
        {none ? "Select tracks" : `${picked.length} selected`}
      </Typography>
      <Tooltip title="Download">
        <span>
          <IconButton aria-label="Download selected" disabled={none} onClick={() => void download()}>
            <DownloadOutlinedIcon />
          </IconButton>
        </span>
      </Tooltip>
      <Tooltip title="Remove downloads">
        <span>
          <IconButton
            aria-label="Remove downloads of selected"
            disabled={downloaded === 0}
            onClick={() => setRemoving(downloadedAmong(picked.map((t) => t.id)))}
          >
            <FileDownloadOffOutlinedIcon />
          </IconButton>
        </span>
      </Tooltip>
      <Tooltip title="Add to queue">
        <span>
          <IconButton aria-label="Add selected to queue" disabled={none} onClick={() => { player.enqueue(picked, "end"); onDone(); }}>
            <PlaylistAddIcon />
          </IconButton>
        </span>
      </Tooltip>
      <Tooltip title="Add to playlist">
        <span>
          <IconButton aria-label="Add selected to playlist" disabled={none} onClick={(e) => setPlaylists(e.currentTarget)}>
            <LibraryAddIcon />
          </IconButton>
        </span>
      </Tooltip>
      <Tooltip title="Done">
        <IconButton aria-label="Stop selecting" onClick={onDone}>
          <CloseIcon />
        </IconButton>
      </Tooltip>
      {removing && <RemoveDownloadsDialog trackIds={removing} onClose={() => setRemoving(null)} onDone={onDone} />}
      <PlaylistMenu tracks={picked} anchor={playlists} onClose={() => setPlaylists(null)} onDone={onDone} />
    </Box>
  );
}
