import { COMMANDS } from "@/lib/commands";
import type { Explore } from "@/lib/types";
import { useCached } from "@/browse/useCached";

const NONE: Explore = { newReleases: [], trending: [], newVideos: [], moods: [] };

/**
 * One list of the explore page. Several shelves read from the same query, so it is made once and
 * shared by its key. The list is undefined while loading and empty when the query failed.
 */
export function useExploreList<K extends keyof Explore>(key: K): Explore[K] | undefined {
  const explore = useCached<Explore | null>("explore", () => COMMANDS.media.explore(), null);
  return explore === undefined ? undefined : (explore ?? NONE)[key];
}
