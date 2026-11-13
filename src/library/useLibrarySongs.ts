import { mapLimit, unique } from "@/browse/tracks";
import { useCached } from "@/browse/useCached";
import { COMMANDS } from "@/lib/commands";
import { useLikedCount } from "@/lib/likes";
import { usePlaylists } from "@/lib/library";
import { useLocalVersion } from "@/lib/local";
import type { Track } from "@/lib/types";
import { loadLiked, loadLocal } from "./play";

/** Playlists read at once. They are local queries, the cap only keeps a large library from opening them all together. */
const AT_ONCE = 4;

/**
 * Every song of the library once, from the liked songs, the local songs and each playlist. The
 * track table of the database also holds songs that left the library, so it cannot answer this.
 * Undefined while loading. The key names every part the list depends on, so an edit loads it again.
 */
export function useLibrarySongs(): Track[] | undefined {
  const playlists = usePlaylists();
  const liked = useLikedCount();
  const local = useLocalVersion();
  const stamp = playlists?.map((p) => `${p.id}:${p.updatedMs}`).join(",");

  return useCached(
    playlists ? `library-songs:${liked}:${local}:${stamp}` : null,
    async () => {
      const [likedSongs, localSongs, lists] = await Promise.all([
        loadLiked(),
        loadLocal(),
        mapLimit([...(playlists ?? [])], AT_ONCE, (p) => COMMANDS.playlists.tracks({ id: p.id })),
      ]);
      return unique([...likedSongs, ...localSongs, ...lists.flat()]);
    },
    [],
  );
}
