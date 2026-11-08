import { useState } from "react";
import { Divider, IconButton, ListItemIcon, ListItemText, Menu, MenuItem, Tooltip } from "@mui/material";
import AddIcon from "@mui/icons-material/Add";
import CheckIcon from "@mui/icons-material/Check";
import LibraryAddIcon from "@mui/icons-material/LibraryAdd";
import { addToPlaylist, createPlaylist, isSavedAlbum, usePlaylists } from "@/lib/library";
import type { Track } from "@/lib/types";
import PlaylistDialog from "./PlaylistDialog";

/** How long the check mark replaces the icon after a track was added. */
const DONE_MS = 1500;

/**
 * A button that opens the playlists of the library to put a track in. A saved album is not
 * offered, it is a copy of the release and not a list to build.
 */
export default function AddToPlaylist({ track }: { track: Track }) {
  const own = usePlaylists()?.filter((p) => !isSavedAlbum(p)) ?? [];
  const [anchor, setAnchor] = useState<HTMLElement | null>(null);
  const [creating, setCreating] = useState(false);
  const [done, setDone] = useState(false);

  const flash = () => {
    setDone(true);
    setTimeout(() => setDone(false), DONE_MS);
  };

  const add = async (id: string) => {
    setAnchor(null);
    if (await addToPlaylist(id, [track])) flash();
  };

  const createAndAdd = async (name: string, description: string | null) => {
    setCreating(false);
    const made = await createPlaylist(name, description);
    if (made && (await addToPlaylist(made.id, [track]))) flash();
  };

  return (
    <>
      <Tooltip title="Add to playlist">
        <IconButton size="small" aria-label="Add to playlist" onClick={(e) => setAnchor(e.currentTarget)}>
          {done ? <CheckIcon fontSize="small" color="primary" /> : <LibraryAddIcon fontSize="small" />}
        </IconButton>
      </Tooltip>
      <Menu
        anchorEl={anchor} open={!!anchor} onClose={() => setAnchor(null)}
        slotProps={{ paper: { sx: { maxHeight: 360, minWidth: 200 } } }}
      >
        <MenuItem onClick={() => { setAnchor(null); setCreating(true); }}>
          <ListItemIcon><AddIcon fontSize="small" /></ListItemIcon>
          <ListItemText>New playlist</ListItemText>
        </MenuItem>
        {own.length > 0 && <Divider />}
        {own.map((p) => (
          <MenuItem key={p.id} onClick={() => void add(p.id)}>
            <ListItemText slotProps={{ primary: { noWrap: true } }}>{p.name}</ListItemText>
          </MenuItem>
        ))}
      </Menu>
      {creating && (
        <PlaylistDialog open title="New playlist" submitLabel="Create" onClose={() => setCreating(false)} onSubmit={(n, d) => void createAndAdd(n, d)} />
      )}
    </>
  );
}
