import { useMemo, useState } from "react";
import { Box, Button, Chip, IconButton, Skeleton, Tooltip, Typography } from "@mui/material";
import CheckIcon from "@mui/icons-material/Check";
import ContentCopyIcon from "@mui/icons-material/ContentCopy";
import PlayArrowIcon from "@mui/icons-material/PlayArrow";
import { useLocation, useParams } from "react-router-dom";
import { duration } from "@/components/media/format";
import { COMMANDS } from "@/lib/commands";
import type { Artist, ArtistPage as Page, Item } from "@/lib/types";
import Cover from "@/player/Cover";
import { player } from "@/player/store";
import { accentFill, accentTonal, eyebrow, GUTTER, heroTitle, radius, sectionTitle, subdued } from "@/themes/ui";
import { IS_MOBILE } from "@/util/platform";
import ItemShelf from "./ItemShelf";
import TrackShelf from "./TrackShelf";
import TrackTable from "./TrackTable";
import { useCached } from "./useCached";
import { useFollow } from "./useFollow";

const AVATAR = IS_MOBILE ? 112 : 160;
const BANNER_HEIGHT = 340;

type Releases = "popular" | "albums" | "singles";
const CHIPS: [Releases, string][] = [["popular", "Popular releases"], ["albums", "Albums"], ["singles", "Singles"]];

/** The provider words the audience as a bare count, like "2.1M", or as a sentence. */
const audienceText = (text: string) => (/^[\d.,]+\s*[KMB]?$/i.test(text) ? `${text} subscribers` : text);

const releaseLine = (i: Item) => [i.releaseYear, i.label].filter(Boolean).join(" · ");

const NO_RELEASES: Record<Releases, string> = {
  popular: "No releases were listed for this artist.",
  albums: "No albums were listed for this artist.",
  singles: "No singles were listed for this artist.",
};

/**
 * The page of an artist. Playlists are the one part that is a search by name, the provider
 * does not list them on the artist.
 */
export default function ArtistPage() {
  const { id = "" } = useParams<{ id: string }>();
  // set when the screen was opened from a listing
  const item = (useLocation().state as { item?: Item } | null)?.item;
  const page = useCached(`artist:${id}`, () => COMMANDS.media.artist({ id }), null);
  const [releases, setReleases] = useState<Releases>("popular");
  const [copied, setCopied] = useState(false);

  const name = page?.name ?? item?.title;
  const thumbnail = page?.thumbnail ?? item?.thumbnail ?? null;
  const artist: Artist | null = name ? { id, name, thumbnail } : null;
  const { following, toggle } = useFollow(artist);

  // waits for the name, so a key of null holds it back
  const playlists = useCached(
    name ? `artist-playlists:${id}` : null,
    () => COMMANDS.media.searchItems({ query: name!, kind: "playlist", limit: 12 }),
    [],
  );

  const shown = useMemo(() => {
    if (!page) return undefined;
    if (releases === "albums") return page.albums;
    if (releases === "singles") return page.singles;
    // the provider gives no popularity for releases, so the newest come first
    return [...page.albums, ...page.singles].sort((a, b) => (b.releaseYear ?? 0) - (a.releaseYear ?? 0));
  }, [page, releases]);

  if (page === null && !item) {
    return <Typography sx={{ ...subdued, px: GUTTER, pt: 12 }}>This artist could not be loaded.</Typography>;
  }

  const of = <T,>(pick: (p: Page) => T[]): T[] | undefined => (page === undefined ? undefined : page ? pick(page) : []);
  const topSongs = page?.topSongs ?? [];
  const tracks = topSongs.map((s) => s.track);
  const hasReleases = !!page && page.albums.length + page.singles.length > 0;

  const copyLink = async () => {
    await navigator.clipboard.writeText(`https://music.youtube.com/channel/${id}`);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <Box sx={{ position: "relative", pb: 2 }}>
      {/* the picture fades into the page, and without one a tint of the accent stands in */}
      <Box
        sx={{
          position: "absolute", inset: "0 0 auto 0", height: BANNER_HEIGHT, backgroundSize: "cover", backgroundPosition: "center 20%",
          backgroundImage: (t) => {
            const fade = `linear-gradient(to bottom, transparent 30%, ${t.palette.background.default} 100%)`;
            return thumbnail
              ? `${fade}, url("${encodeURI(thumbnail)}")`
              : `${fade}, linear-gradient(135deg, ${t.palette.primary.dark}, transparent)`;
          },
        }}
      />

      <Box sx={{ position: "relative", pt: `${BANNER_HEIGHT - AVATAR - 24}px`, px: GUTTER }}>
        <Box sx={{ display: "flex", alignItems: "flex-end", gap: 3 }}>
          <Cover src={thumbnail} px={480} sx={{ width: AVATAR, height: AVATAR, borderRadius: "50%", fontSize: 56, boxShadow: 8 }} />
          <Box sx={{ minWidth: 0, pb: 1 }}>
            <Typography sx={eyebrow}>ARTIST</Typography>
            {name ? (
              <Typography sx={heroTitle}>{name}</Typography>
            ) : (
              <Skeleton width={280} height={64} />
            )}
            {page?.audience && <Typography sx={subdued}>{audienceText(page.audience)}</Typography>}
          </Box>
        </Box>

        <Box sx={{ display: "flex", alignItems: "center", gap: 1.5, py: 3 }}>
          <IconButton
            aria-label="Play" disabled={tracks.length === 0} onClick={() => player.play(tracks, 0)}
            sx={{ width: 52, height: 52, ...accentFill }}
          >
            <PlayArrowIcon />
          </IconButton>
          <Button
            variant="outlined" size="small" disabled={following === null} onClick={() => void toggle()}
            sx={{ borderRadius: radius.pill, px: 2.5, fontWeight: 700, ...(following && accentTonal) }}
          >
            {following ? "Following" : "Follow"}
          </Button>
          <Tooltip title={copied ? "Link copied" : "Copy link"}>
            <IconButton aria-label="Copy link" onClick={() => void copyLink()}>
              {copied ? <CheckIcon fontSize="small" /> : <ContentCopyIcon fontSize="small" />}
            </IconButton>
          </Tooltip>
        </Box>
      </Box>

      {(page === undefined || topSongs.length > 0) && (
        <Box component="section" sx={{ pb: 3 }}>
          <Typography sx={{ ...sectionTitle, px: GUTTER, pb: 1.5 }}>Popular</Typography>
          {page ? (
            <TrackTable tracks={tracks} plays={topSongs.map((s) => s.plays)} onPlay={(i) => player.play(tracks, i)} />
          ) : (
            Array.from({ length: 5 }, (_, i) => <Skeleton key={i} variant="rounded" height={44} sx={{ mx: GUTTER, my: 0.75 }} />)
          )}
        </Box>
      )}

      <ItemShelf
        title="Discography"
        items={shown}
        empty={NO_RELEASES[releases]}
        // a filter with nothing in it keeps the shelf and its chips, or there would be no way back
        hideWhenEmpty={!hasReleases}
        byline={releaseLine}
        controls={
          hasReleases && (
            <Box sx={{ display: "flex", gap: 1, pt: 1 }} role="group" aria-label="Releases">
              {CHIPS.map(([value, text]) => (
                <Chip
                  key={value} label={text} clickable size="small" variant="outlined"
                  sx={releases === value ? accentTonal : undefined}
                  onClick={() => setReleases(value)}
                />
              ))}
            </Box>
          )
        }
      />
      <ItemShelf title="Playlists" items={playlists} empty="" hideWhenEmpty />
      <TrackShelf
        title="Music Videos"
        tracks={of((p) => p.videos)}
        empty=""
        variant="wide"
        subtitle={(t) => duration(t.durationSecs)}
        hideWhenEmpty
      />
      <ItemShelf title="Fans might also like" items={of((p) => p.related)} empty="" hideWhenEmpty />
    </Box>
  );
}
