import { Box, Skeleton, Typography } from "@mui/material";
import SearchIcon from "@mui/icons-material/Search";
import { useSearchParams } from "react-router-dom";
import { duration } from "@/components/media/format";
import { COMMANDS } from "@/lib/commands";
import { player } from "@/player/store";
import { GUTTER, sectionTitle, subdued } from "@/themes/ui";
import ItemShelf from "./ItemShelf";
import TrackShelf from "./TrackShelf";
import TrackTable from "./TrackTable";
import { useCached } from "./useCached";

const SONGS = 10;
const ARTISTS = 5;
const SHELF = 12;

/**
 * Each section loads on its own, so the first one to arrive shows at once. The loads queue in
 * `useCached`, which keeps a search from starting a dozen provider processes together.
 */
export default function SearchResults() {
  const query = (useSearchParams()[0].get("q") ?? "").trim();
  const q = query.toLowerCase();

  const artists = useCached(query ? `search-artists:${q}` : null, () => COMMANDS.media.searchItems({ query, kind: "artist", limit: ARTISTS }), []);
  const songs = useCached(query ? `search-songs:${q}` : null, () => COMMANDS.media.search({ query, limit: SONGS }), []);
  const albums = useCached(query ? `search-albums:${q}` : null, () => COMMANDS.media.searchItems({ query, kind: "album", limit: SHELF }), []);
  const playlists = useCached(query ? `search-playlists:${q}` : null, () => COMMANDS.media.searchItems({ query, kind: "playlist", limit: SHELF }), []);
  const videos = useCached(query ? `search-videos:${q}` : null, () => COMMANDS.media.searchVideos({ query, limit: SHELF }), []);

  if (!query) {
    return (
      <Box sx={{ pt: 20, textAlign: "center", ...subdued }}>
        <SearchIcon sx={{ fontSize: 64 }} />
        <Typography>Search for songs, artists, albums, or playlists</Typography>
      </Box>
    );
  }

  const all = [artists, songs, albums, playlists, videos];
  if (all.every((r) => r && r.length === 0)) {
    return <Typography sx={{ ...subdued, px: GUTTER, pt: 12 }}>Nothing found for “{query}”.</Typography>;
  }

  return (
    <Box sx={{ pt: 10, pb: 2 }}>
      <ItemShelf title="Artists" items={artists} empty="" hideWhenEmpty />

      {(songs === undefined || songs.length > 0) && (
        <Box component="section" sx={{ pb: 3 }}>
          <Typography sx={{ ...sectionTitle, px: GUTTER, pb: 1.5 }}>Songs</Typography>
          {songs ? (
            <TrackTable tracks={songs} onPlay={(i) => player.play(songs, i)} />
          ) : (
            Array.from({ length: 6 }, (_, i) => <Skeleton key={i} variant="rounded" height={44} sx={{ mx: GUTTER, my: 0.75 }} />)
          )}
        </Box>
      )}

      <ItemShelf title="Albums" items={albums} empty="" hideWhenEmpty />
      <ItemShelf title="Playlists" items={playlists} empty="" hideWhenEmpty />
      <TrackShelf
        title="Videos"
        tracks={videos}
        empty=""
        variant="wide"
        subtitle={(t) => duration(t.durationSecs)}
        hideWhenEmpty
      />
    </Box>
  );
}
