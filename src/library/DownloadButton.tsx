import { useState } from "react";
import {
  Button, CircularProgress, Dialog, DialogActions, DialogContent, DialogContentText, DialogTitle, IconButton, Tooltip,
} from "@mui/material";
import DownloadDoneIcon from "@mui/icons-material/DownloadDone";
import DownloadOutlinedIcon from "@mui/icons-material/DownloadOutlined";
import { downloadTrack, isLocalFile, removeDownload, useDownloadStatus } from "@/lib/local";
import type { Track } from "@/lib/types";
import { useBackGuard } from "@/router/useBackGuard";

/**
 * The question before a download is deleted, since the song then has to be downloaded or
 * streamed again. It is apart from the button so a menu can ask it as well.
 */
export function RemoveDownloadDialog({ track, onClose }: { track: Track; onClose: () => void }) {
  useBackGuard(true, onClose);

  const remove = async () => {
    onClose();
    await removeDownload(track.id);
  };

  return (
    <Dialog open onClose={onClose} fullWidth maxWidth="xs">
      <DialogTitle>Remove download</DialogTitle>
      <DialogContent>
        <DialogContentText>
          {`"${track.title}" will be deleted from this device. It stays in the playlists it is in.`}
        </DialogContentText>
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>Cancel</Button>
        <Button color="error" variant="contained" onClick={() => void remove()}>Remove</Button>
      </DialogActions>
    </Dialog>
  );
}

/**
 * Saves a song on this device, and once it is saved, marks it and takes the file off again. The
 * mark is the button, so a song that is downloaded can be recognised in any list and removed from
 * the same spot. A song that comes from a folder of the user has no button, it is on the device
 * already.
 */
export default function DownloadButton({ track }: { track: Track }) {
  const status = useDownloadStatus(track.id);
  const [confirming, setConfirming] = useState(false);

  if (isLocalFile(track.id)) return null;

  return (
    <>
      {status === "busy" && (
        <Tooltip title="Downloading...">
          <span>
            <IconButton size="small" aria-label="Downloading" disabled>
              <CircularProgress size={16} />
            </IconButton>
          </span>
        </Tooltip>
      )}
      {status === "none" && (
        <Tooltip title="Download">
          <IconButton size="small" aria-label="Download" onClick={() => void downloadTrack(track)}>
            <DownloadOutlinedIcon fontSize="small" />
          </IconButton>
        </Tooltip>
      )}
      {status === "done" && (
        <Tooltip title="Downloaded, press to remove">
          <IconButton size="small" aria-label="Remove download" color="primary" onClick={() => setConfirming(true)}>
            <DownloadDoneIcon fontSize="small" />
          </IconButton>
        </Tooltip>
      )}
      {confirming && <RemoveDownloadDialog track={track} onClose={() => setConfirming(false)} />}
    </>
  );
}
