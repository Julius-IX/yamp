import { MouseEvent, useState } from "react";
import { Box, ButtonBase, ListItemButton, ListItemText, Menu, MenuItem, Tooltip } from "@mui/material";
import AlbumIcon from "@mui/icons-material/Album";
import PlayArrowIcon from "@mui/icons-material/PlayArrow";

export type AlbumSummary = { id: string; title: string; trackCount: number; cover?: string };

// Stand-in until the library exists. Long enough to show that the list scrolls on its own.
const useSidebarAlbums = (): AlbumSummary[] =>
  Array.from({ length: 12 }, (_, i) => ({ id: String(i + 1), title: `Album ${i + 1}`, trackCount: 8 + i }));

/**
 * An album row has more than one target. The row opens the album, the cover plays it and a
 * right click opens a menu. The cover button stops the click so it never also opens the album.
 */
function AlbumItem({
  album, compact, onOpen, onPlay, onMenu,
}: {
  album: AlbumSummary;
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
        "&:hover .album-play": { opacity: 1 },
      }}
    >
      <Box
        sx={{
          position: "relative", flexShrink: 0, width: 40, height: 40, borderRadius: 1, overflow: "hidden",
          bgcolor: "action.selected", display: "flex", alignItems: "center", justifyContent: "center",
        }}
      >
        {album.cover ? (
          <Box component="img" src={album.cover} alt="" sx={{ width: "100%", height: "100%", objectFit: "cover" }} />
        ) : (
          <AlbumIcon color="disabled" />
        )}
        {!compact && (
          <ButtonBase
            className="album-play"
            aria-label={`Play ${album.title}`}
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
          primary={album.title}
          secondary={`${album.trackCount} tracks`}
          slotProps={{ primary: { noWrap: true }, secondary: { noWrap: true } }}
          sx={{ m: 0, minWidth: 0 }}
        />
      )}
    </ListItemButton>
  );

  return compact ? <Tooltip title={album.title} placement="right">{row}</Tooltip> : row;
}

export default function AlbumList({ compact }: { compact: boolean }) {
  const albums = useSidebarAlbums();
  const [menu, setMenu] = useState<{ album: AlbumSummary; x: number; y: number } | null>(null);

  // Neither an album screen nor playback exists yet.
  const open = (_a: AlbumSummary) => { };
  const play = (_a: AlbumSummary) => { };

  return (
    <>
      {albums.map((a) => (
        <AlbumItem
          key={a.id}
          album={a}
          compact={compact}
          onOpen={() => open(a)}
          onPlay={() => play(a)}
          onMenu={(e) => setMenu({ album: a, x: e.clientX, y: e.clientY })}
        />
      ))}
      <Menu
        open={!!menu}
        onClose={() => setMenu(null)}
        anchorReference="anchorPosition"
        anchorPosition={menu ? { top: menu.y, left: menu.x } : undefined}
      >
        <MenuItem onClick={() => { play(menu!.album); setMenu(null); }}>Play</MenuItem>
        <MenuItem onClick={() => { open(menu!.album); setMenu(null); }}>Open</MenuItem>
      </Menu>
    </>
  );
}
