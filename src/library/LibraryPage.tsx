import { useState } from "react";
import { Box, Button, Chip, Typography } from "@mui/material";
import AddIcon from "@mui/icons-material/Add";
import { useNavigate } from "react-router-dom";
import { Album } from "@/browse/paths";
import MediaCard, { CardSkeleton } from "@/browse/MediaCard";
import { coverOf, createPlaylist, isSavedAlbum, usePlaylists } from "@/lib/library";
import { useLikedCount } from "@/lib/likes";
import type { LibraryPlaylist } from "@/lib/types";
import { accentTonal, GUTTER, subdued, surface } from "@/themes/ui";
import { IS_MOBILE } from "@/util/platform";
import LikedArt from "./LikedArt";
import { LikedSongs, OwnPlaylist } from "./paths";
import PlaylistDialog from "./PlaylistDialog";
import { playLiked, playPlaylist, queueLiked, queuePlaylist } from "./play";

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
 * Everything saved in one grid, filtered by kind. The liked songs come first and cannot be
 * removed, which is why they are not part of the playlists the grid lists.
 */
export default function LibraryPage() {
  const navigate = useNavigate();
  const playlists = usePlaylists();
  const likedCount = useLikedCount();
  const [filter, setFilter] = useState<Filter>("all");
  const [creating, setCreating] = useState(false);

  const shown = playlists?.filter((p) => (filter === "all" ? true : isSavedAlbum(p) === (filter === "albums")));

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
            key={f.id} label={f.label} clickable onClick={() => setFilter(f.id)}
            variant="outlined" sx={f.id === filter ? accentTonal : surface}
          />
        ))}
        <Button size="small" startIcon={<AddIcon />} onClick={() => setCreating(true)} sx={{ ml: "auto" }}>New playlist</Button>
      </Box>

      <Box sx={{ display: "flex", flexWrap: "wrap", gap: 1.5, px: GUTTER }}>
        {filter !== "albums" && (
          <MediaCard
            art={<LikedArt sx={{ width: "100%", aspectRatio: "1", fontSize: 56 }} />}
            title="Liked Songs"
            subtitle={`Playlist · ${likedCount} songs`}
            onClick={() => navigate(LikedSongs.to())}
            onPlay={() => void playLiked()}
            onQueue={() => void queueLiked()}
          />
        )}
        {shown
          ? shown.map((p) => <PlaylistCard key={p.id} playlist={p} />)
          : Array.from({ length: GHOSTS }, (_, i) => <CardSkeleton key={i} />)}
      </Box>
      {shown?.length === 0 && EMPTY[filter] && <Typography sx={{ ...subdued, px: GUTTER, pt: 1 }}>{EMPTY[filter]}</Typography>}

      <PlaylistDialog
        open={creating} title="New playlist" submitLabel="Create"
        onClose={() => setCreating(false)} onSubmit={(n, d) => void create(n, d)}
      />
    </Box>
  );
}
