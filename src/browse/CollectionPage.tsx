import { useEffect, useState } from "react";
import { Box, IconButton, Skeleton, Tooltip, Typography } from "@mui/material";
import AddIcon from "@mui/icons-material/Add";
import CheckIcon from "@mui/icons-material/Check";
import PlayArrowIcon from "@mui/icons-material/PlayArrow";
import ShuffleIcon from "@mui/icons-material/Shuffle";
import { useLocation, useParams } from "react-router-dom";
import { COMMANDS } from "@/lib/commands";
import { refreshPlaylists } from "@/lib/library";
import { logger } from "@/lib/log";
import type { Item, Track } from "@/lib/types";
import Cover from "@/player/Cover";
import { player } from "@/player/store";
import { accentFill, eyebrow, GUTTER, heroTitle, radius, subdued } from "@/themes/ui";
import { IS_MOBILE } from "@/util/platform";
import TrackTable from "./TrackTable";
import { useCached } from "./useCached";

const log = logger("browse");
const COVER = IS_MOBILE ? 140 : 200;

const shuffled = (tracks: Track[]) => {
  const out = [...tracks];
  for (let i = out.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [out[i], out[j]] = [out[j], out[i]];
  }
  return out;
};

/**
 * An album or a playlist from the provider, as opposed to one in the library, so there is
 * nothing to edit. The tracks can be liked, and the collection itself can be saved. Either one
 * becomes a library playlist with a copy of its tracks, since the library has no notion of
 * following a provider collection. An album keeps its own id for the playlist. That is how this
 * page tells that the album is saved, and saving it twice cannot make two copies.
 */
export default function CollectionPage({ kind }: { kind: "album" | "playlist" }) {
  const { id = "" } = useParams<{ id: string }>();
  // set when the screen was opened from a listing, it fills the header before the query is back
  const item = (useLocation().state as { item?: Item } | null)?.item;
  // only the one for this kind loads, a null key holds the other back
  const album = useCached(kind === "album" ? `album:${id}` : null, () => COMMANDS.media.album({ id }), null);
  const playlist = useCached(kind === "playlist" ? `playlist:${id}` : null, () => COMMANDS.media.playlist({ id }), null);
  const data = kind === "album" ? album : playlist;
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    if (kind !== "album") return setSaved(false);
    let stale = false;
    COMMANDS.playlists.get({ id }).then((p) => !stale && setSaved(p !== null)).catch(() => {});
    return () => {
      stale = true;
    };
  }, [kind, id]);

  if (data === null) {
    return <Typography sx={{ ...subdued, px: GUTTER, pt: 12 }}>This {kind} could not be loaded.</Typography>;
  }

  const tracks = data?.tracks ?? [];
  const title = data?.title || item?.title;
  const cover = album?.thumbnail ?? item?.thumbnail ?? tracks[0]?.thumbnail;
  const artist = album?.artist ?? item?.subtitle ?? undefined;
  const year = album?.year ?? item?.releaseYear;
  const meta = (kind === "album" ? [artist, year] : [item?.subtitle]).concat(data ? `${tracks.length} songs` : undefined).filter(Boolean).join(" · ");
  const label = (album?.releaseType ?? item?.label ?? kind).toUpperCase();

  const toggleSave = async () => {
    if (!data || !title) return;
    try {
      if (kind === "album") {
        if (saved) {
          await COMMANDS.playlists.delete({ id });
        } else {
          // a playlist shows the thumbnail of its first track, and the tracks of an album can come without one
          const withCover = tracks.map((t) => (t.thumbnail ? t : { ...t, thumbnail: cover ?? null }));
          await COMMANDS.playlists.saveAs({ id, name: title, description: [artist, year].filter(Boolean).join(" · ") || null, tracks: withCover });
        }
        setSaved(!saved);
        void refreshPlaylists();
      } else if (!saved) {
        const created = await COMMANDS.playlists.create({ name: title });
        await COMMANDS.playlists.addTracks({ id: created.id, tracks });
        setSaved(true);
        void refreshPlaylists();
      }
    } catch (e) {
      log.warn(`couldn't save "${title}":`, e);
    }
  };

  const saveLabel = kind === "album" ? (saved ? "Remove from library" : "Save to library") : saved ? "Saved to library" : "Save a copy to library";
  const ready = !!data && tracks.length > 0;

  return (
    <Box sx={{ pt: IS_MOBILE ? 10 : 9, pb: 2 }}>
      <Box sx={{ display: "flex", flexDirection: IS_MOBILE ? "column" : "row", alignItems: IS_MOBILE ? "flex-start" : "flex-end", gap: 3, px: GUTTER, pb: 3 }}>
        <Cover src={cover} px={640} sx={{ width: COVER, height: COVER, borderRadius: radius.card, fontSize: 64, boxShadow: 8 }} />
        <Box sx={{ minWidth: 0 }}>
          <Typography sx={eyebrow}>{label}</Typography>
          {title ? (
            <Typography sx={heroTitle}>{title}</Typography>
          ) : (
            <Skeleton width={320} height={56} />
          )}
          <Typography sx={{ ...subdued, mt: 0.5 }}>{meta || "\u00A0"}</Typography>
        </Box>
      </Box>

      <Box sx={{ display: "flex", alignItems: "center", gap: 1.5, px: GUTTER, pb: 2 }}>
        <IconButton
          aria-label="Play" disabled={!ready} onClick={() => player.play(tracks, 0)}
          sx={{ width: 52, height: 52, ...accentFill }}
        >
          <PlayArrowIcon />
        </IconButton>
        <Tooltip title="Shuffle">
          <span>
            <IconButton aria-label="Shuffle" disabled={!ready} onClick={() => player.play(shuffled(tracks), 0)}>
              <ShuffleIcon />
            </IconButton>
          </span>
        </Tooltip>
        <Tooltip title={saveLabel}>
          <span>
            <IconButton aria-label={saveLabel} disabled={!ready || (kind === "playlist" && saved)} onClick={() => void toggleSave()}>
              {saved ? <CheckIcon /> : <AddIcon />}
            </IconButton>
          </span>
        </Tooltip>
      </Box>

      {data ? (
        <TrackTable tracks={tracks} onPlay={(i) => player.play(tracks, i)} />
      ) : (
        Array.from({ length: 8 }, (_, i) => <Skeleton key={i} variant="rounded" height={44} sx={{ mx: GUTTER, my: 0.75 }} />)
      )}
    </Box>
  );
}
