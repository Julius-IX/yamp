import { useEffect, useMemo, useState } from "react";
import { Box, IconButton, Skeleton, Tooltip, Typography } from "@mui/material";
import DeleteOutlinedIcon from "@mui/icons-material/DeleteOutlined";
import EditIcon from "@mui/icons-material/Edit";
import HideImageIcon from "@mui/icons-material/HideImage";
import { useLocation, useNavigate } from "react-router-dom";
import TrackTable from "@/browse/TrackTable";
import { COMMANDS } from "@/lib/commands";
import { editPlaylist, moveInPlaylist, removeFromPlaylist, removePlaylistCover, usePlaylists } from "@/lib/library";
import { logger } from "@/lib/log";
import type { Track } from "@/lib/types";
import { player } from "@/player/store";
import { useRouteParams } from "@/router/hooks";
import { TAB_ROOT } from "@/router/paths";
import { GUTTER, subdued } from "@/themes/ui";
import { IS_MOBILE } from "@/util/platform";
import DeletePlaylistDialog from "./DeletePlaylistDialog";
import CoverPicker from "./CoverPicker";
import Header, { PlayButtons } from "./Header";
import { OwnPlaylist } from "./paths";
import PlaylistDialog from "./PlaylistDialog";
import PlaylistSort, { DEFAULT_SORT, sortLabel, sortOrder, type SortState } from "./PlaylistSort";

const log = logger("library");

/**
 * A playlist of the library, which can be renamed, emptied track by track and deleted. The tracks
 * are read again whenever the playlist changes. Its `updatedMs` moves with every edit, so it
 * tells this screen when to read, also when the edit came from another device.
 */
export default function OwnPlaylistPage() {
  const { id } = useRouteParams(OwnPlaylist);
  const navigate = useNavigate();
  const canGoBack = useLocation().key !== "default";
  const playlists = usePlaylists();
  const playlist = playlists?.find((p) => p.id === id);
  const stamp = playlist?.updatedMs;

  const [tracks, setTracks] = useState<Track[] | null>(null);
  const [editing, setEditing] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [sort, setSort] = useState<SortState>(DEFAULT_SORT);
  // bumped when a move failed, so the tracks are read again and show what the playlist really holds
  const [reload, setReload] = useState(0);

  // the sort belongs to the playlist on screen, another one starts in its own order
  useEffect(() => setSort(DEFAULT_SORT), [id]);

  // positions in the playlist, in the order shown. Removing a row needs its position in the playlist
  const order = useMemo(() => sortOrder(tracks ?? [], sort), [tracks, sort]);
  const shown = useMemo(() => order.map((i) => tracks![i]), [order, tracks]);

  useEffect(() => {
    if (stamp === undefined) return;
    let stale = false;
    COMMANDS.playlists
      .tracks({ id })
      .then((t) => !stale && setTracks(t))
      .catch((e) => {
        log.warn(`couldn't load the playlist ${id}:`, e);
        if (!stale) setTracks([]);
      });
    return () => {
      stale = true;
    };
  }, [id, stamp, reload]);

  // a deleted playlist leaves nothing to show, so the screen goes back to where it was opened from
  const leave = () => (canGoBack ? navigate(-1) : navigate(TAB_ROOT, { replace: true }));

  if (playlists && !playlist) {
    return <Typography sx={{ ...subdued, px: GUTTER, pt: 12 }}>This playlist could not be found.</Typography>;
  }

  // the row jumps at once and the backend follows, waiting for its answer would show the old order
  const move = (from: number, to: number) => {
    setTracks((t) => {
      if (!t) return t;
      const next = [...t];
      next.splice(to, 0, ...next.splice(from, 1));
      return next;
    });
    void moveInPlaylist(id, from, to).then((ok) => !ok && setReload((n) => n + 1));
  };

  const sorted = sort.key !== "order";
  const meta = playlist && [playlist.description, `${playlist.trackCount} songs`].filter(Boolean).join(" · ");

  return (
    <Box sx={{ pt: IS_MOBILE ? 10 : 9, pb: 2 }}>
      <Header
        art={<CoverPicker playlist={playlist} />}
        label="PLAYLIST"
        title={playlist?.name}
        meta={meta}
      >
        <PlayButtons tracks={shown} />
        <PlaylistSort sort={sort} onChange={setSort} />
        <Tooltip title="Edit details">
          <span>
            <IconButton aria-label="Edit details" disabled={!playlist} onClick={() => setEditing(true)}><EditIcon /></IconButton>
          </span>
        </Tooltip>
        {playlist?.coverVersion != null && (
          <Tooltip title="Remove custom cover">
            <IconButton aria-label="Remove custom cover" onClick={() => void removePlaylistCover(id)}><HideImageIcon /></IconButton>
          </Tooltip>
        )}
        <Tooltip title="Delete playlist">
          <span>
            <IconButton aria-label="Delete playlist" disabled={!playlist} onClick={() => setDeleting(true)}><DeleteOutlinedIcon /></IconButton>
          </span>
        </Tooltip>
      </Header>

      {tracks === null && Array.from({ length: 8 }, (_, i) => <Skeleton key={i} variant="rounded" height={44} sx={{ mx: GUTTER, my: 0.75 }} />)}
      {tracks?.length === 0 && (
        <Typography sx={{ ...subdued, px: GUTTER }}>This playlist is empty. The playlist button on a song adds it here.</Typography>
      )}
      {sorted && (
        <Typography variant="caption" sx={{ ...subdued, px: GUTTER, pb: 1, display: "block" }}>
          Sorted by {sortLabel(sort.key).toLowerCase()}. Rows can be moved again in the playlist order.
        </Typography>
      )}
      {tracks && tracks.length > 0 && (
        <TrackTable tracks={shown} onPlay={(i) => player.play(shown, i)} onRemove={(i) => void removeFromPlaylist(id, order[i])}
          // a sorted view is not the order of the playlist, so there is nothing to rearrange in it
          onMove={sorted || shown.length < 2 ? undefined : move}
        />
      )}

      <PlaylistDialog
        open={editing}
        title="Edit playlist"
        submitLabel="Save"
        initial={playlist && { name: playlist.name, description: playlist.description }}
        onClose={() => setEditing(false)}
        onSubmit={(name, description) => {
          setEditing(false);
          void editPlaylist(id, name, description);
        }}
      />
      <DeletePlaylistDialog
        playlist={deleting && playlist ? { id, name: playlist.name } : null}
        onClose={() => setDeleting(false)}
        onDeleted={leave}
      />
    </Box>
  );
}
