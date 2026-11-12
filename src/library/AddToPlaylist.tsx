import { useState } from "react";
import { Divider, IconButton, ListItemIcon, ListItemText, Menu, MenuItem, Tooltip } from "@mui/material";
import AddIcon from "@mui/icons-material/Add";
import CheckIcon from "@mui/icons-material/Check";
import LibraryAddIcon from "@mui/icons-material/LibraryAdd";
import { addToPlaylist, createPlaylist, isSavedAlbum, usePlaylists } from "@/lib/library";
import type { Track } from "@/lib/types";
import PlaylistDialog from "./PlaylistDialog";

/**
 * The playlists of the library to put tracks in, as a menu below `anchor`. It is apart from the
 * button so that a menu of actions or a selection of many tracks can open it as well. A saved
 * album is not offered, it is a copy of the release and not a list to build. `onDone` runs once
 * the tracks are in.
 */
export function PlaylistMenu({ tracks, anchor, onClose, onDone }: {
  tracks: Track[];
  anchor: HTMLElement | null;
  onClose: () => void;
  onDone?: () => void;
}) {
  const own = usePlaylists()?.filter((p) => !isSavedAlbum(p)) ?? [];
  const [creating, setCreating] = useState(false);

  const add = async (id: string) => {
    onClose();
    if (await addToPlaylist(id, tracks)) onDone?.();
  };

  const createAndAdd = async (name: string, description: string | null) => {
    setCreating(false);
    const made = await createPlaylist(name, description);
    if (made && (await addToPlaylist(made.id, tracks))) onDone?.();
  };

  return (
    <>
      <Menu
        anchorEl={anchor} open={!!anchor} onClose={onClose}
        slotProps={{ paper: { sx: { maxHeight: 360, minWidth: 200 } } }}
      >
        <MenuItem onClick={() => { onClose(); setCreating(true); }}>
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

/** How long the check mark replaces the icon after a track was added. */
const DONE_MS = 1500;

/**
 * A button that opens the playlists of the library to put a track in. The size follows the buttons
 * it sits next to, a row of tracks uses the small one and a player the medium one.
 */
export default function AddToPlaylist({ track, size = "small" }: { track: Track; size?: "small" | "medium" }) {
  const [anchor, setAnchor] = useState<HTMLElement | null>(null);
  const [done, setDone] = useState(false);

  const flash = () => {
    setDone(true);
    setTimeout(() => setDone(false), DONE_MS);
  };

  return (
    <>
      <Tooltip title="Add to playlist">
        <IconButton size={size} aria-label="Add to playlist" onClick={(e) => setAnchor(e.currentTarget)}>
          {done ? <CheckIcon fontSize={size} color="primary" /> : <LibraryAddIcon fontSize={size} />}
        </IconButton>
      </Tooltip>
      <PlaylistMenu tracks={[track]} anchor={anchor} onClose={() => setAnchor(null)} onDone={flash} />
    </>
  );
}
