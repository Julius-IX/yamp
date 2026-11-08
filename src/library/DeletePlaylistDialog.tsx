import { Button, Dialog, DialogActions, DialogContent, DialogContentText, DialogTitle } from "@mui/material";
import { deletePlaylist } from "@/lib/library";
import { useBackGuard } from "@/router/useBackGuard";

/** Asks first, since the playlist cannot be brought back. The songs stay in the library. */
export default function DeletePlaylistDialog({
  playlist, onClose, onDeleted,
}: {
  /** The playlist to delete, or null while the dialog is closed. */
  playlist: { id: string; name: string } | null;
  onClose: () => void;
  onDeleted?: () => void;
}) {
  useBackGuard(!!playlist, onClose);

  const confirm = async () => {
    if (!playlist) return;
    const done = await deletePlaylist(playlist.id);
    onClose();
    if (done) onDeleted?.();
  };

  return (
    <Dialog open={!!playlist} onClose={onClose} fullWidth maxWidth="xs">
      <DialogTitle>Delete playlist</DialogTitle>
      <DialogContent>
        <DialogContentText>
          {playlist ? `"${playlist.name}" will be removed from the library.` : ""}
        </DialogContentText>
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>Cancel</Button>
        <Button color="error" variant="contained" onClick={() => void confirm()}>Delete</Button>
      </DialogActions>
    </Dialog>
  );
}
