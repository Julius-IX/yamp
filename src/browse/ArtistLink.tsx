import { Fragment, type KeyboardEvent, type MouseEvent, type ReactNode } from "react";
import { Box } from "@mui/material";
import { COMMANDS } from "@/lib/commands";
import { logger } from "@/lib/log";
import type { Item, Track } from "@/lib/types";
import { isRadio } from "@/radio/station";
import { useTabs } from "@/router/TabsContext";
import { Search } from "@/search/paths";
import { Artist } from "./paths";

const log = logger("browse");

/** Names resolved so far. A track carries its artist as text, so the page is found by searching. */
const resolved = new Map<string, Promise<Item | null>>();

function resolveArtist(name: string): Promise<Item | null> {
  const key = name.toLowerCase();
  let hit = resolved.get(key);
  if (!hit) {
    hit = COMMANDS.media
      .searchItems({ query: name, kind: "artist", limit: 3 })
      // the best match is the artist named exactly like this, and failing that the first one
      .then((items) => items.find((i) => i.title.toLowerCase() === key) ?? items[0] ?? null);
    resolved.set(key, hit);
    // a failed or empty lookup is not remembered, the next click tries again
    hit.then((item) => item ?? resolved.delete(key)).catch(() => resolved.delete(key));
  }
  return hit;
}

/** "A, B" and "A & B" are separate artists. */
const names = (artist: string) => artist.split(/,\s*|\s+&\s+/).filter(Boolean);

/**
 * The artist of a track as links to their pages. It renders inline, so it goes inside whatever
 * text element shows the artist. The events stop here because it often sits in a row that plays
 * when clicked. `onOpen` runs on a click, for a screen that has to get out of the way first.
 */
export default function ArtistLink({ artist, onOpen }: { artist: string; onOpen?: () => void }) {
  const { openBrowse } = useTabs();

  const open = async (name: string) => {
    onOpen?.();
    try {
      const item = await resolveArtist(name);
      // the item rides along like it does from a card, so the page has a name and a cover at once
      if (item) return openBrowse(Artist.to({ id: item.id }), { item });
    } catch (e) {
      log.warn(`couldn't find the artist "${name}":`, e);
    }
    // a name without a page of its own still leads somewhere, the results of a search for it
    openBrowse(Search.to({ query: { q: name } }));
  };

  return (
    <>
      {names(artist).map((name, i) => (
        <Fragment key={`${name}:${i}`}>
          {i > 0 && ", "}
          <Box
            component="span"
            role="link"
            tabIndex={0}
            onClick={(e: MouseEvent) => {
              e.stopPropagation();
              void open(name);
            }}
            onKeyDown={(e: KeyboardEvent) => {
              if (e.key !== "Enter") return;
              e.stopPropagation();
              void open(name);
            }}
            sx={{ cursor: "pointer", "&:hover, &:focus-visible": { color: "text.primary", textDecoration: "underline" } }}
          >
            {name}
          </Box>
        </Fragment>
      ))}
    </>
  );
}

/**
 * The artist line of a track. A station carries its country and bitrate there, which is not an
 * artist that has a page to look up.
 */
export function TrackArtist({ track, onOpen }: { track: Track; onOpen?: () => void }) {
  return isRadio(track.id) ? <>{track.artist}</> : <ArtistLink artist={track.artist} onOpen={onOpen} />;
}

/**
 * The line under a card for a release. The artist is a link and the other parts follow after
 * a dot. Parts that are empty are left out, and so is the dot.
 */
export function ArtistLine({ artist, rest }: { artist: string | null; rest: (string | number | null | undefined)[] }): ReactNode {
  const tail = rest.filter(Boolean).join(" · ");
  return (
    <>
      {artist && <ArtistLink artist={artist} />}
      {artist && tail && " · "}
      {tail}
    </>
  );
}
