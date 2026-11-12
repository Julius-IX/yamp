import { MouseEvent, useState } from "react";
import { Box, ButtonBase, ListItemButton, ListItemText, Menu, MenuItem, Tooltip } from "@mui/material";
import PlayArrowIcon from "@mui/icons-material/PlayArrow";
import { applyOrder, useOrder } from "@/lib/arrange";
import { coverOf, isSavedAlbum, usePlaylists } from "@/lib/library";
import { useLikedCount } from "@/lib/likes";
import DeletePlaylistDialog from "@/library/DeletePlaylistDialog";
import LikedArt from "@/library/LikedArt";
import { LikedSongs, OwnPlaylist } from "@/library/paths";
import { playLiked, playPlaylist } from "@/library/play";
import Cover from "@/player/Cover";
import { useTabs } from "@/router/TabsContext";

type Entry = { id: string; title: string; trackCount: number; cover?: string | null; liked?: boolean };

/**
 * A playlist row has more than one target. The row opens the playlist, the cover plays it and a
 * right click opens a menu. The cover button stops the click so it never also opens the playlist.
 */
function PlaylistItem({
  entry, compact, onOpen, onPlay, onMenu,
}: {
  entry: Entry;
  compact: boolean;
  onOpen: () => void;
  onPlay: () => void;
  onMenu: (e: MouseEvent) => void;
}) {
  const row = (
    <ListItemButton
      onClick={onOpen}
      onContextMenu={(e) => { e.preventDefault(); onMenu(e); }}
      sx={{
        gap: 1.5, px: 1, py: 0.75, borderRadius: 1,
        justifyContent: compact ? "center" : "flex-start",
        "&:hover .playlist-play": { opacity: 1 },
      }}
    >
      <Box sx={{ position: "relative", flexShrink: 0, width: 40, height: 40, borderRadius: 1, overflow: "hidden" }}>
        {entry.liked ? (
          <LikedArt sx={{ width: "100%", height: "100%", fontSize: 20 }} />
        ) : (
          <Cover src={entry.cover} px={96} sx={{ width: "100%", height: "100%", fontSize: 20 }} />
        )}
        {!compact && (
          <ButtonBase
            className="playlist-play"
            aria-label={`Play ${entry.title}`}
            onClick={(e) => { e.stopPropagation(); onPlay(); }}
            sx={{
              position: "absolute", inset: 0, bgcolor: "rgba(0,0,0,0.55)", color: "#fff", opacity: 0, transition: "opacity 0.2s",
              // without hover the overlay could only be hit by accident
              "@media (hover: none)": { display: "none" },
            }}
          >
            <PlayArrowIcon />
          </ButtonBase>
        )}
      </Box>
      {!compact && (
        <ListItemText
          primary={entry.title}
          secondary={`${entry.trackCount} tracks`}
          slotProps={{ primary: { noWrap: true }, secondary: { noWrap: true } }}
          sx={{ m: 0, minWidth: 0 }}
        />
      )}
    </ListItemButton>
  );

  return compact ? <Tooltip title={entry.title} placement="right">{row}</Tooltip> : row;
}

/**
 * The liked songs and the playlists of the library, in the order of the library screen. They open in the library tab, whatever tab is
 * showing. Saved albums are left out, they live in the library screen.
 */
export default function PlaylistList({ compact }: { compact: boolean }) {
  const { openTab } = useTabs();
  const playlists = usePlaylists();
  const saved = useOrder("playlists");
  const likedCount = useLikedCount();
  const [menu, setMenu] = useState<{ entry: Entry; x: number; y: number } | null>(null);
  const [deleting, setDeleting] = useState<Entry | null>(null);

  const entries: Entry[] = [
    { id: LikedSongs.path, title: "Liked Songs", trackCount: likedCount, liked: true },
    ...(playlists ? applyOrder(playlists, saved ?? []) : []).filter((p) => !isSavedAlbum(p)).map((p) => ({ id: p.id, title: p.name, trackCount: p.trackCount, cover: coverOf(p) })),
  ];

  const open = (e: Entry) => openTab("library", e.liked ? LikedSongs.to() : OwnPlaylist.to({ id: e.id }));
  const play = (e: Entry) => void (e.liked ? playLiked() : playPlaylist(e.id));

  return (
    <>
      {entries.map((e) => (
        <PlaylistItem
          key={e.id}
          entry={e}
          compact={compact}
          onOpen={() => open(e)}
          onPlay={() => play(e)}
          onMenu={(ev) => setMenu({ entry: e, x: ev.clientX, y: ev.clientY })}
        />
      ))}
      <Menu
        open={!!menu}
        onClose={() => setMenu(null)}
        anchorReference="anchorPosition"
        anchorPosition={menu ? { top: menu.y, left: menu.x } : undefined}
      >
        <MenuItem onClick={() => { play(menu!.entry); setMenu(null); }}>Play</MenuItem>
        <MenuItem onClick={() => { open(menu!.entry); setMenu(null); }}>Open</MenuItem>
        {/* the liked songs cannot be deleted */}
        {menu && !menu.entry.liked && (
          <MenuItem onClick={() => { setDeleting(menu.entry); setMenu(null); }}>Delete</MenuItem>
        )}
      </Menu>
      <DeletePlaylistDialog playlist={deleting && { id: deleting.id, name: deleting.title }} onClose={() => setDeleting(null)} />
    </>
  );
}
