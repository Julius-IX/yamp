import { useState } from "react";
import { IconButton, Tooltip } from "@mui/material";
import AddIcon from "@mui/icons-material/Add";
import { createPlaylist } from "@/lib/library";
import { OwnPlaylist } from "@/library/paths";
import PlaylistDialog from "@/library/PlaylistDialog";
import { useTabs } from "@/router/TabsContext";

/** Makes a playlist and opens it, so songs can be added right away. */
export default function NewPlaylistButton() {
  const { openTab } = useTabs();
  const [open, setOpen] = useState(false);

  const create = async (name: string, description: string | null) => {
    setOpen(false);
    const made = await createPlaylist(name, description);
    if (made) openTab("library", OwnPlaylist.to({ id: made.id }));
  };

  return (
    <>
      <Tooltip title="New playlist">
        <IconButton size="small" aria-label="New playlist" onClick={() => setOpen(true)}>
          <AddIcon fontSize="small" />
        </IconButton>
      </Tooltip>
      <PlaylistDialog open={open} title="New playlist" submitLabel="Create" onClose={() => setOpen(false)} onSubmit={(n, d) => void create(n, d)} />
    </>
  );
}
