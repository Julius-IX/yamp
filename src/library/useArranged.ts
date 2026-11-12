import { useMemo, useState } from "react";
import { applyOrder, saveOrder, useOrder, type OrderSlot } from "@/lib/arrange";
import type { Track } from "@/lib/types";
import { DEFAULT_SORT, sortOrder } from "./PlaylistSort";

/**
 * Sorting and rearranging for a list that has no playlist of its own, the liked and the local
 * songs. A move is saved as the order of the whole list. A sort only changes what is shown, so
 * rows cannot be moved in it, as in the playlists of the library. `tracks` stays null until
 * both the songs and the saved order are loaded, so the list never shows in one order first.
 */
export function useArranged(slot: OrderSlot, tracks: Track[] | null) {
  const saved = useOrder(slot);
  const [sort, setSort] = useState(DEFAULT_SORT);

  const arranged = useMemo(() => (tracks && saved ? applyOrder(tracks, saved) : null), [tracks, saved]);
  const shown = useMemo(() => {
    if (!arranged) return null;
    return sortOrder(arranged, sort).map((i) => arranged[i]);
  }, [arranged, sort]);

  const sorted = sort.key !== "order";
  const move = (from: number, to: number) => {
    const ids = arranged!.map((t) => t.id);
    ids.splice(to, 0, ...ids.splice(from, 1));
    saveOrder(slot, ids);
  };

  return { shown, sort, setSort, sorted, move: sorted || (shown?.length ?? 0) < 2 ? undefined : move };
}
