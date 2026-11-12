import { useState } from "react";
import {
  Button, CircularProgress, Dialog, DialogActions, DialogContent, DialogContentText, DialogTitle, IconButton, Tooltip,
} from "@mui/material";
import DownloadDoneIcon from "@mui/icons-material/DownloadDone";
import DownloadOutlinedIcon from "@mui/icons-material/DownloadOutlined";
import { downloadTrack, isLocalFile, removeDownload, removeDownloads, useDownloadStatus } from "@/lib/local";
import type { Track } from "@/lib/types";
import { useBackGuard } from "@/router/useBackGuard";

/** The question before something is removed. Downloads in particular need to be downloaded or streamed again. */
export function RemovalDialog({ title, message, onConfirm, onClose }: {
  title: string;
  message: string;
  onConfirm: () => Promise<void> | void;
  onClose: () => void;
}) {
  useBackGuard(true, onClose);

  const confirm = async () => {
    onClose();
    await onConfirm();
  };

  return (
    <Dialog open onClose={onClose} fullWidth maxWidth="xs">
      <DialogTitle>{title}</DialogTitle>
      <DialogContent>
        <DialogContentText>{message}</DialogContentText>
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>Cancel</Button>
        <Button color="error" variant="contained" onClick={() => void confirm()}>Remove</Button>
      </DialogActions>
    </Dialog>
  );
}

/** The question for one song. It is apart from the button so a menu can ask it as well. */
export function RemoveDownloadDialog({ track, onClose }: { track: Track; onClose: () => void }) {
  return (
    <RemovalDialog
      title="Remove download"
      message={`"${track.title}" will be deleted from this device. It stays in the playlists it is in.`}
      onConfirm={() => removeDownload(track.id)}
      onClose={onClose}
    />
  );
}

/** The question for several songs at once. `onDone` runs when the removal is confirmed. */
export function RemoveDownloadsDialog({ trackIds, onClose, onDone }: {
  trackIds: string[];
  onClose: () => void;
  onDone?: () => void;
}) {
  const many = trackIds.length !== 1;
  return (
    <RemovalDialog
      title={many ? "Remove downloads" : "Remove download"}
      message={`${trackIds.length} ${many ? "songs" : "song"} will be deleted from this device. ${many ? "They stay" : "It stays"} in the playlists ${many ? "they are" : "it is"} in.`}
      onConfirm={() => {
        onDone?.();
        return removeDownloads(trackIds);
      }}
      onClose={onClose}
    />
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
