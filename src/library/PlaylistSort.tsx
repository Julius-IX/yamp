import { useState } from "react";
import { IconButton, ListItemIcon, ListItemText, Menu, MenuItem, Tooltip } from "@mui/material";
import ArrowDownwardIcon from "@mui/icons-material/ArrowDownward";
import ArrowUpwardIcon from "@mui/icons-material/ArrowUpward";
import CheckIcon from "@mui/icons-material/Check";
import SortIcon from "@mui/icons-material/Sort";
import type { Track } from "@/lib/types";

/** `order` is the order of the playlist itself, which is the only one that can be rearranged. */
export type SortKey = "order" | "title" | "artist" | "album" | "duration" | "year";
export type SortState = { key: SortKey; descending: boolean };

export const DEFAULT_SORT: SortState = { key: "order", descending: false };

const OPTIONS: { key: SortKey; label: string }[] = [
  { key: "order", label: "Playlist order" },
  { key: "title", label: "Title" },
  { key: "artist", label: "Artist" },
  { key: "album", label: "Album" },
  { key: "duration", label: "Duration" },
  { key: "year", label: "Release year" },
];

export const sortLabel = (key: SortKey) => OPTIONS.find((o) => o.key === key)!.label;

const value = (t: Track, key: SortKey): string | number | null => {
  switch (key) {
    case "title": return t.title;
    case "artist": return t.artist;
    case "album": return t.album;
    case "duration": return t.durationSecs;
    case "year": return t.releaseYear;
    default: return null;
  }
};

const text = new Intl.Collator(undefined, { sensitivity: "base", numeric: true });

/**
 * The positions of `tracks` in the order to show them. Positions are returned and not tracks,
 * since a playlist can hold a track twice and removing needs the position in the playlist. A
 * track without a value for the key goes last in both directions, and equal ones keep their
 * playlist order.
 */
export function sortOrder(tracks: Track[], { key, descending }: SortState): number[] {
  const order = tracks.map((_, i) => i);
  if (key === "order") return order;
  const sign = descending ? -1 : 1;
  return order.sort((i, j) => {
    const [a, b] = [value(tracks[i], key), value(tracks[j], key)];
    if (a === null || b === null) return a === b ? i - j : a === null ? 1 : -1;
    const cmp = typeof a === "string" && typeof b === "string" ? text.compare(a, b) : Number(a) - Number(b);
    return cmp * sign || i - j;
  });
}

/**
 * A button that opens the ways to sort a playlist. Choosing the active key again flips its
 * direction. The sort only changes what is shown, the playlist keeps its own order.
 */
export default function PlaylistSort({ sort, onChange }: { sort: SortState; onChange: (sort: SortState) => void }) {
  const [anchor, setAnchor] = useState<HTMLElement | null>(null);
  const active = sort.key !== "order";

  const choose = (key: SortKey) => {
    setAnchor(null);
    if (key === "order") return onChange(DEFAULT_SORT);
    onChange(key === sort.key ? { key, descending: !sort.descending } : { key, descending: false });
  };

  return (
    <>
      <Tooltip title="Sort">
        <IconButton aria-label="Sort" color={active ? "primary" : "default"} onClick={(e) => setAnchor(e.currentTarget)}>
          <SortIcon />
        </IconButton>
      </Tooltip>
      <Menu anchorEl={anchor} open={!!anchor} onClose={() => setAnchor(null)}>
        {OPTIONS.map((o) => {
          const selected = o.key === sort.key;
          return (
            <MenuItem key={o.key} selected={selected} onClick={() => choose(o.key)}>
              <ListItemText>{o.label}</ListItemText>
              {/* the arrow shows the direction, and the check marks the playlist order */}
              {selected && (
                <ListItemIcon sx={{ minWidth: 0, ml: 2 }}>
                  {o.key === "order" ? <CheckIcon fontSize="small" /> : sort.descending ? <ArrowDownwardIcon fontSize="small" /> : <ArrowUpwardIcon fontSize="small" />}
                </ListItemIcon>
              )}
            </MenuItem>
          );
        })}
      </Menu>
    </>
  );
}
