import { useEffect, useState } from "react";
import { Button, Dialog, DialogActions, DialogContent, DialogTitle, TextField } from "@mui/material";
import { useBackGuard } from "@/router/useBackGuard";

/** Name and description of a playlist, for making one and for editing one. */
export default function PlaylistDialog({
  open, title, submitLabel, initial, onClose, onSubmit,
}: {
  open: boolean;
  title: string;
  submitLabel: string;
  initial?: { name: string; description: string | null };
  onClose: () => void;
  onSubmit: (name: string, description: string | null) => void;
}) {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  useBackGuard(open, onClose);

  // the fields start over on every open, a closed dialog stays mounted for its exit animation
  useEffect(() => {
    if (!open) return;
    setName(initial?.name ?? "");
    setDescription(initial?.description ?? "");
  }, [open]);

  const valid = name.trim().length > 0;
  const submit = () => valid && onSubmit(name.trim(), description.trim() || null);

  return (
    <Dialog open={open} onClose={onClose} fullWidth maxWidth="xs">
      <DialogTitle>{title}</DialogTitle>
      <DialogContent sx={{ display: "flex", flexDirection: "column", gap: 2, pt: "8px !important" }}>
        <TextField
          autoFocus label="Name" value={name} onChange={(e) => setName(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && submit()} fullWidth
        />
        <TextField
          label="Description" value={description} onChange={(e) => setDescription(e.target.value)}
          multiline minRows={2} maxRows={4} fullWidth
        />
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>Cancel</Button>
        <Button variant="contained" disabled={!valid} onClick={submit}>{submitLabel}</Button>
      </DialogActions>
    </Dialog>
  );
}
