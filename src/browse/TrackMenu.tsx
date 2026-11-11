import { useState, type ReactNode } from "react";
import { Box, Divider, IconButton, ListItemIcon, ListItemText, Menu, MenuItem, Tooltip, type SxProps, type Theme } from "@mui/material";
import CloseIcon from "@mui/icons-material/Close";
import DownloadDoneIcon from "@mui/icons-material/DownloadDone";
import DownloadOutlinedIcon from "@mui/icons-material/DownloadOutlined";
import DragIndicatorIcon from "@mui/icons-material/DragIndicator";
import FavoriteBorderIcon from "@mui/icons-material/FavoriteBorder";
import FavoriteIcon from "@mui/icons-material/Favorite";
import LibraryAddIcon from "@mui/icons-material/LibraryAdd";
import MoreVertIcon from "@mui/icons-material/MoreVert";
import OndemandVideoIcon from "@mui/icons-material/OndemandVideo";
import PlayArrowIcon from "@mui/icons-material/PlayArrow";
import PlaylistAddIcon from "@mui/icons-material/PlaylistAdd";
import PlaylistPlayIcon from "@mui/icons-material/PlaylistPlay";
import { toggleLike, useIsLiked } from "@/lib/likes";
import { downloadTrack, isLocalFile, useDownloadStatus } from "@/lib/local";
import type { Track } from "@/lib/types";
import { player } from "@/player/store";
import { isRadio } from "@/radio/station";
import { PlaylistMenu } from "@/library/AddToPlaylist";
import { RemoveDownloadDialog } from "@/library/DownloadButton";
import { video } from "@/video/store";

type Props = {
  track: Track;
  /** Where the menu opens. It is closed while this is null. */
  anchor: HTMLElement | null;
  onClose: () => void;
  /** Adds a "Play" entry. For a card whose click does something else, like opening a video. */
  onPlay?: () => void;
  /** What "Watch video" does. Without it only this track is played and shown. */
  onWatch?: () => void;
  /** Adds the like entry. A row has a heart of its own, a card does not. */
  like?: boolean;
  /** The entries after the line, for what only makes sense in a list. */
  onRemove?: () => void;
  /** Shows the handles to drag the tracks of the list the track is in into another order. */
  onReorder?: () => void;
};

function Item({ icon, label, onClick }: { icon: ReactNode; label: string; onClick: () => void }) {
  return (
    <MenuItem onClick={onClick}>
      <ListItemIcon>{icon}</ListItemIcon>
      <ListItemText>{label}</ListItemText>
    </MenuItem>
  );
}

/**
 * What can be done with a track, for where there is no room for a button of each. The menu, the
 * playlist menu it leads to and the dialog stay mounted while it is closed, since they outlive
 * the click that opened them. A line sets what applies to any track apart from what only applies
 * inside a list.
 */
export default function TrackMenu({ track, anchor, onClose, onPlay, onWatch, like, onRemove, onReorder }: Props) {
  const liked = useIsLiked(track.id);
  const download = useDownloadStatus(track.id);
  const [playlists, setPlaylists] = useState<HTMLElement | null>(null);
  const [removing, setRemoving] = useState(false);

  const radio = isRadio(track.id);
  const local = isLocalFile(track.id);
  // runs the action after the menu is gone
  const act = (fn: () => void) => () => {
    onClose();
    fn();
  };

  return (
    <>
      <Menu anchorEl={anchor} open={!!anchor} onClose={onClose} slotProps={{ paper: { sx: { minWidth: 220 } } }}>
        {onPlay && <Item icon={<PlayArrowIcon fontSize="small" />} label="Play" onClick={act(onPlay)} />}
        {!radio && !local && (
          <Item icon={<OndemandVideoIcon fontSize="small" />} label="Watch video" onClick={act(onWatch ?? (() => video.watch([track])))} />
        )}
        <Item icon={<PlaylistPlayIcon fontSize="small" />} label="Play next" onClick={act(() => player.enqueue([track], "next"))} />
        <Item icon={<PlaylistAddIcon fontSize="small" />} label="Add to queue" onClick={act(() => player.enqueue([track], "end"))} />
        <Item
          icon={<LibraryAddIcon fontSize="small" />}
          label="Add to playlist"
          onClick={() => {
            setPlaylists(anchor);
            onClose();
          }}
        />
        {!radio && !local && download === "none" && (
          <Item icon={<DownloadOutlinedIcon fontSize="small" />} label="Download" onClick={act(() => void downloadTrack(track))} />
        )}
        {download === "done" && (
          <Item icon={<DownloadDoneIcon fontSize="small" />} label="Remove download" onClick={act(() => setRemoving(true))} />
        )}
        {like && !radio && (
          <Item
            icon={liked ? <FavoriteIcon fontSize="small" /> : <FavoriteBorderIcon fontSize="small" />}
            label={liked ? "Remove from liked songs" : "Add to liked songs"}
            onClick={act(() => void toggleLike(track))}
          />
        )}
        {(onRemove || onReorder) && <Divider />}
        {onRemove && <Item icon={<CloseIcon fontSize="small" />} label="Remove from playlist" onClick={act(onRemove)} />}
        {onReorder && <Item icon={<DragIndicatorIcon fontSize="small" />} label="Reorder tracks" onClick={act(onReorder)} />}
      </Menu>
      <PlaylistMenu tracks={[track]} anchor={playlists} onClose={() => setPlaylists(null)} />
      {removing && <RemoveDownloadDialog track={track} onClose={() => setRemoving(false)} />}
    </>
  );
}

/**
 * The three dots that open the menu of a track. React sends the clicks of a menu and its dialogs
 * up the component tree and not the DOM one, so they would reach what the button sits on, like
 * a card that opens on a click. The wrapper keeps them here and adds no box of its own.
 */
export function TrackMenuButton({ sx, ...menu }: Omit<Props, "anchor" | "onClose"> & { sx?: SxProps<Theme> }) {
  const [anchor, setAnchor] = useState<HTMLElement | null>(null);
  return (
    <Box component="span" sx={{ display: "contents" }} onClick={(e) => e.stopPropagation()}>
      <Tooltip title="More">
        <IconButton size="small" aria-label="More actions" sx={sx} onClick={(e) => setAnchor(e.currentTarget)}>
          <MoreVertIcon fontSize="small" />
        </IconButton>
      </Tooltip>
      <TrackMenu {...menu} anchor={anchor} onClose={() => setAnchor(null)} />
    </Box>
  );
}
