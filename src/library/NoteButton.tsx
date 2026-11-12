import { useEffect, useState } from "react";
import { Button, Dialog, DialogActions, DialogContent, DialogTitle, IconButton, TextField, Tooltip } from "@mui/material";
import EditNoteIcon from "@mui/icons-material/EditNote";
import { saveNote, useNote, type NoteSlot } from "@/lib/arrange";
import { useBackGuard } from "@/router/useBackGuard";

/**
 * Edits the description of the liked or the local songs, the one thing of a playlist made in the
 * app that these two lacked. It is kept as a note and not on a playlist row, since they are no
 * rows. Saving an empty text removes it.
 */
export default function NoteButton({ slot }: { slot: NoteSlot }) {
  const note = useNote(slot);
  const [open, setOpen] = useState(false);
  const [text, setText] = useState("");
  const close = () => setOpen(false);
  useBackGuard(open, close);

  // the field starts from the saved note on every open, a closed dialog stays mounted for its exit animation
  useEffect(() => {
    if (open) setText(note ?? "");
  }, [open]);

  return (
    <>
      <Tooltip title="Edit description">
        <span>
          <IconButton aria-label="Edit description" disabled={note === null} onClick={() => setOpen(true)}><EditNoteIcon /></IconButton>
        </span>
      </Tooltip>
      <Dialog open={open} onClose={close} fullWidth maxWidth="xs">
        <DialogTitle>Edit description</DialogTitle>
        <DialogContent sx={{ pt: "8px !important" }}>
          <TextField autoFocus label="Description" value={text} onChange={(e) => setText(e.target.value)} multiline minRows={2} maxRows={6} fullWidth />
        </DialogContent>
        <DialogActions>
          <Button onClick={close}>Cancel</Button>
          <Button variant="contained" onClick={() => { saveNote(slot, text); close(); }}>Save</Button>
        </DialogActions>
      </Dialog>
    </>
  );
}
