import { useMemo, useRef, useState } from "react";
import { Box, Button, Chip, IconButton, Typography, type Theme } from "@mui/material";
import AddIcon from "@mui/icons-material/Add";
import CheckIcon from "@mui/icons-material/Check";
import SwapVertIcon from "@mui/icons-material/SwapVert";
import { useNavigate } from "react-router-dom";
import { Album } from "@/browse/paths";
import MediaCard, { CardSkeleton } from "@/browse/MediaCard";
import { Handle } from "@/browse/TrackTable";
import { useDragReorder } from "@/browse/useDragReorder";
import { applyOrder, saveOrder, useOrder } from "@/lib/arrange";
import { coverOf, createPlaylist, isSavedAlbum, usePlaylists } from "@/lib/library";
import { useLikedCount } from "@/lib/likes";
import { useLocalCount } from "@/lib/local";
import type { LibraryPlaylist } from "@/lib/types";
import { accentTonal, GUTTER, radius, scrim, subdued, surface } from "@/themes/ui";
import { IS_MOBILE } from "@/util/platform";
import LikedArt from "./LikedArt";
import LocalArt from "./LocalArt";
import { LikedSongs, LocalSongs, OwnPlaylist } from "./paths";
import PlaylistDialog from "./PlaylistDialog";
import { playLiked, playLocal, playPlaylist, queueLiked, queueLocal, queuePlaylist } from "./play";

type Filter = "all" | "playlists" | "albums";

const FILTERS: { id: Filter; label: string }[] = [
  { id: "all", label: "All" },
  { id: "playlists", label: "Playlists" },
  { id: "albums", label: "Albums" },
];

const EMPTY: Record<Filter, string> = {
  all: "",
  playlists: "Playlists that are made show up here.",
  albums: "Albums that are saved show up here.",
};

const GHOSTS = 6;

/** A saved album opens as the album, since its tracks are a copy of a release. A playlist opens in the library. */
function PlaylistCard({ playlist }: { playlist: LibraryPlaylist }) {
  const navigate = useNavigate();
  const album = isSavedAlbum(playlist);
  return (
    <MediaCard
      fluid={IS_MOBILE}
      thumbnail={coverOf(playlist)}
      title={playlist.name}
      subtitle={album ? ["Album", playlist.description].filter(Boolean).join(" · ") : `Playlist · ${playlist.trackCount} songs`}
      onClick={() => navigate(album ? Album.to({ id: playlist.id }) : OwnPlaylist.to({ id: playlist.id }))}
      onPlay={() => void playPlaylist(playlist.id)}
      onQueue={() => void queuePlaylist(playlist.id)}
    />
  );
}

/**
 * Everything saved in one grid, filtered by kind. The liked songs and the local songs come first
 * and cannot be removed, which is why they are not part of the playlists the grid lists. For
 * the same reason they cannot be moved. The other cards are moved with the drag handle that
 * the rows of a playlist have too. Only the unfiltered grid can be rearranged, a filtered one
 * holds just part of the order. A touch screen has no hover to show the handles, so a button
 * there turns them on.
 */
export default function LibraryPage() {
  const navigate = useNavigate();
  const playlists = usePlaylists();
  const likedCount = useLikedCount();
  const localCount = useLocalCount();
  const [filter, setFilter] = useState<Filter>("all");
  const [creating, setCreating] = useState(false);
  const [handles, setHandles] = useState(false);
  const saved = useOrder("playlists");
  const grid = useRef<HTMLDivElement>(null);

  const ordered = useMemo(() => playlists && saved && applyOrder(playlists, saved), [playlists, saved]);
  const shown = ordered?.filter((p) => (filter === "all" ? true : isSavedAlbum(p) === (filter === "albums")));
  const canMove = filter === "all" && (ordered?.length ?? 0) > 1;

  const move = (from: number, to: number) => {
    const ids = ordered!.map((p) => p.id);
    ids.splice(to, 0, ...ids.splice(from, 1));
    saveOrder("playlists", ids);
  };

  // the card the pointer is in, or else the nearest one, since the gaps of the grid belong to no card
  const indexAt = (x: number, y: number) => {
    const cards = Array.from(grid.current?.querySelectorAll<HTMLElement>("[data-card]") ?? []);
    let best = 0;
    let nearest = Infinity;
    cards.forEach((c, i) => {
      const r = c.getBoundingClientRect();
      const d = Math.hypot(x - (r.left + r.width / 2), y - (r.top + r.height / 2));
      if (d < nearest) [best, nearest] = [i, d];
    });
    return best;
  };
  const { drag, start } = useDragReorder(grid, canMove ? move : undefined, indexAt);

  const create = async (name: string, description: string | null) => {
    setCreating(false);
    const made = await createPlaylist(name, description);
    if (made) navigate(OwnPlaylist.to({ id: made.id }));
  };

  return (
    // the search bar floats over the top, so on a narrow screen the title sits below it
    <Box sx={{ pt: IS_MOBILE ? 10 : 3, pb: 2, marginBottom: 1 }}>
      {/* the button is not in this row, the floating search bar covers its right half on a narrow window */}
      <Box sx={{ px: GUTTER, pb: 2 }}>
        <Typography variant={IS_MOBILE ? "h5" : "h4"} sx={{ fontWeight: 800, lineHeight: "40px" }}>Library</Typography>
      </Box>

      <Box sx={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: 1, px: GUTTER, pb: 3 }}>
        {FILTERS.map((f) => (
          <Chip
            key={f.id} label={f.label} clickable onClick={() => { setFilter(f.id); setHandles(false); }}
            variant="outlined" sx={f.id === filter ? accentTonal : surface}
          />
        ))}
        {/* symbols only on mobile, so both buttons fit on the row of the filters */}
        {IS_MOBILE && canMove && (
          <IconButton
            size="small" color="primary" aria-label={handles ? "Done" : "Reorder"}
            onClick={() => setHandles(!handles)} sx={{ ml: "auto" }}
          >
            {handles ? <CheckIcon /> : <SwapVertIcon />}
          </IconButton>
        )}
        {IS_MOBILE ? (
          <IconButton
            size="small" color="primary" aria-label="New playlist"
            onClick={() => setCreating(true)} sx={canMove ? undefined : { ml: "auto" }}
          >
            <AddIcon />
          </IconButton>
        ) : (
          <Button size="small" startIcon={<AddIcon />} onClick={() => setCreating(true)} sx={{ ml: "auto" }}>New playlist</Button>
        )}
      </Box>

      <Box
        ref={grid}
        sx={{
          gap: 1.5, px: GUTTER, userSelect: drag ? "none" : "auto",
          // fixed width cards do not fit three across on a phone, so the columns set the width there
          ...(IS_MOBILE ? { display: "grid", gridTemplateColumns: "repeat(3, minmax(0, 1fr))" } : { display: "flex", flexWrap: "wrap" }),
        }}
      >
        {filter !== "albums" && (
          <MediaCard
            fluid={IS_MOBILE}
            art={<LikedArt sx={{ width: "100%", aspectRatio: "1", fontSize: 56 }} />}
            title="Liked Songs"
            subtitle={`Playlist · ${likedCount} songs`}
            onClick={() => navigate(LikedSongs.to())}
            onPlay={() => void playLiked()}
            onQueue={() => void queueLiked()}
          />
        )}
        {filter !== "albums" && (
          <MediaCard
            fluid={IS_MOBILE}
            art={<LocalArt sx={{ width: "100%", aspectRatio: "1", fontSize: 56 }} />}
            title="Local"
            subtitle={`Playlist · ${localCount} songs`}
            onClick={() => navigate(LocalSongs.to())}
            onPlay={() => void playLocal()}
            onQueue={() => void queueLocal()}
          />
        )}
        {/* keeps the playlists that are made off the first row, which only holds the two fixed ones */}
        {IS_MOBILE && filter !== "albums" && <Box aria-hidden />}
        {shown
          ? shown.map((p, i) => (
            <Box
              key={p.id}
              data-card
              sx={{
                position: "relative", borderRadius: radius.card, transition: "opacity 0.15s",
                opacity: drag?.from === i ? 0.4 : 1,
                "&:hover .card-handle": { opacity: 1 },
                // the card paints over its wrapper, so the target is marked by a bar in the gap on
                // the side the dragged card would land on, and an outline that shows the slot
                ...(drag && drag.from !== drag.to && drag.to === i && {
                  outline: "2px dashed", outlineColor: "primary.main", outlineOffset: 2,
                  "&::after": {
                    content: '""', position: "absolute", top: 0, bottom: 0, width: 4, borderRadius: 2, zIndex: 3,
                    [drag.to > drag.from ? "right" : "left"]: -9,
                    bgcolor: "primary.main", boxShadow: (t: Theme) => `0 0 10px 2px ${t.palette.primary.main}`,
                  },
                }),
              }}
            >
              <PlaylistCard playlist={p} />
              {canMove && (
                <Box
                  className="card-handle"
                  sx={{ position: "absolute", top: 12, left: 12, borderRadius: "50%", opacity: IS_MOBILE ? (handles ? 1 : 0) : 0, pointerEvents: IS_MOBILE && !handles ? "none" : "auto", "& [data-handle]": { color: "inherit" }, ...scrim }}
                >
                  <Handle onStart={start(i)} />
                </Box>
              )}
            </Box>
          ))
          : Array.from({ length: GHOSTS }, (_, i) => <CardSkeleton key={i} fluid={IS_MOBILE} />)}
      </Box>
      {shown?.length === 0 && EMPTY[filter] && <Typography sx={{ ...subdued, px: GUTTER, pt: 1 }}>{EMPTY[filter]}</Typography>}

      <PlaylistDialog
        open={creating} title="New playlist" submitLabel="Create"
        onClose={() => setCreating(false)} onSubmit={(n, d) => void create(n, d)}
      />
    </Box>
  );
}
