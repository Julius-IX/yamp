import { Fragment, type KeyboardEvent, type MouseEvent } from "react";
import { Box } from "@mui/material";
import { COMMANDS } from "@/lib/commands";
import { logger } from "@/lib/log";
import { useTabs } from "@/router/TabsContext";
import { Artist } from "./paths";

const log = logger("home");

/** Names resolved so far. A track carries its artist as text, so the page is found by searching. */
const resolved = new Map<string, Promise<string | null>>();

function resolveArtist(name: string): Promise<string | null> {
  const key = name.toLowerCase();
  let hit = resolved.get(key);
  if (!hit) {
    hit = COMMANDS.media
      .searchItems({ query: name, kind: "artist", limit: 3 })
      // the best match is the artist named exactly like this, and failing that the first one
      .then((items) => (items.find((i) => i.title.toLowerCase() === key) ?? items[0])?.id ?? null);
    resolved.set(key, hit);
    // a failed or empty lookup is not remembered, the next click tries again
    hit.then((id) => id ?? resolved.delete(key)).catch(() => resolved.delete(key));
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
  const { openTab } = useTabs();

  const open = async (name: string) => {
    onOpen?.();
    try {
      const id = await resolveArtist(name);
      if (id) openTab("home", Artist.to({ id }));
    } catch (e) {
      log.warn(`couldn't find the artist "${name}":`, e);
    }
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
