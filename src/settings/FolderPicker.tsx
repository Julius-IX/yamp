import { useEffect, useState } from "react";
import {
  Button, Dialog, DialogActions, DialogContent, DialogTitle, List, ListItemButton, ListItemIcon, ListItemText, Typography,
} from "@mui/material";
import ArrowUpwardIcon from "@mui/icons-material/ArrowUpward";
import FolderIcon from "@mui/icons-material/Folder";
import { COMMANDS } from "@/lib/commands";
import { logger } from "@/lib/log";
import type { LocalFolder } from "@/lib/types";
import { useBackGuard } from "@/router/useBackGuard";
import { subdued } from "@/themes/ui";

const log = logger("settings");

/**
 * Chooses a folder by walking through the folders of the device. A phone has no system picker
 * that hands back a path, only one that hands back a permission for a place the app cannot read
 * with plain file access, so the folders are listed here.
 */
export default function FolderPicker({ onPick, onClose }: { onPick: (path: string) => void; onClose: () => void }) {
  const [folder, setFolder] = useState<LocalFolder | null>(null);
  const [at, setAt] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  useBackGuard(true, onClose);

  useEffect(() => {
    let stale = false;
    COMMANDS.local
      .browse({ path: at })
      .then((f) => !stale && (setFolder(f), setError(null)))
      .catch((e) => {
        log.warn("couldn't open the folder:", e);
        if (!stale) setError(String(e));
      });
    return () => {
      stale = true;
    };
  }, [at]);

  return (
    <Dialog open onClose={onClose} fullWidth maxWidth="xs">
      <DialogTitle sx={{ wordBreak: "break-all", fontSize: "1rem" }}>{folder?.path ?? "Choose a folder"}</DialogTitle>
      <DialogContent dividers sx={{ p: 0, minHeight: 240 }}>
        <List dense disablePadding>
          {folder?.parent != null && (
            <ListItemButton onClick={() => setAt(folder.parent)}>
              <ListItemIcon sx={{ minWidth: 40 }}><ArrowUpwardIcon /></ListItemIcon>
              <ListItemText primary="Up" />
            </ListItemButton>
          )}
          {folder?.dirs.map((d) => (
            <ListItemButton key={d.path} onClick={() => setAt(d.path)}>
              <ListItemIcon sx={{ minWidth: 40 }}><FolderIcon /></ListItemIcon>
              <ListItemText primary={d.name} slotProps={{ primary: { noWrap: true } }} />
            </ListItemButton>
          ))}
        </List>
        {folder?.dirs.length === 0 && <Typography sx={{ ...subdued, p: 2 }}>No folders in here.</Typography>}
        {error && <Typography color="error" sx={{ p: 2 }}>{error}</Typography>}
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>Cancel</Button>
        <Button variant="contained" disabled={!folder} onClick={() => folder && onPick(folder.path)}>Use this folder</Button>
      </DialogActions>
    </Dialog>
  );
}
