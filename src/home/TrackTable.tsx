import { Box, IconButton, Tooltip, Typography } from "@mui/material";
import PlayArrowIcon from "@mui/icons-material/PlayArrow";
import PlaylistAddIcon from "@mui/icons-material/PlaylistAdd";
import { useIsLiked } from "@/lib/likes";
import type { Track } from "@/lib/types";
import Cover from "@/player/Cover";
import LikeButton from "@/player/LikeButton";
import { currentTrack, player, usePlayer } from "@/player/store";
import { IS_MOBILE } from "@/util/platform";
import ArtistLink from "./ArtistLink";
import { columnHead, faint, GUTTER, radius, subdued } from "@/themes/ui";

// The header and the rows share these so the columns line up. The buttons sit after them.
const columns = (plays: boolean) =>
  IS_MOBILE ? "32px minmax(0, 1fr)" : `40px minmax(0, 1.4fr) minmax(0, 1fr)${plays ? " 96px" : ""}`;
const BUTTON = 40;
const ACTIONS_WIDTH = BUTTON * 2;

type RowProps = { track: Track; index: number; active: boolean; plays?: string | null; showPlays: boolean; onPlay: () => void };

/**
 * The row is a div that plays when clicked, not a button. The artist and the actions inside are
 * controls of their own, and a button cannot hold another one.
 */
function Row({ track, index, active, plays, showPlays, onPlay }: RowProps) {
  const liked = useIsLiked(track.id);
  return (
    <Box
      role="button"
      tabIndex={0}
      onClick={onPlay}
      onKeyDown={(e) => {
        if (e.target !== e.currentTarget || (e.key !== "Enter" && e.key !== " ")) return;
        e.preventDefault();
        onPlay();
      }}
      sx={{
        display: "flex", alignItems: "center", py: 0.75, borderRadius: radius.row, cursor: "pointer",
        "&:hover, &:focus-visible": { bgcolor: "action.hover", outline: "none" },
        "&:hover .row-index": { display: "none" },
        "&:hover .row-play": { display: "flex" },
        "&:hover .row-action": { opacity: 1 },
      }}
    >
      <Box sx={{ flex: 1, minWidth: 0, display: "grid", gridTemplateColumns: columns(showPlays), alignItems: "center" }}>
        <Typography className="row-index" variant="body2" sx={{ ...(active ? { color: "primary.main" } : faint), textAlign: "center" }}>
          {index + 1}
        </Typography>
        <Box className="row-play" sx={{ display: "none", justifyContent: "center", color: "text.primary" }}>
          <PlayArrowIcon fontSize="small" />
        </Box>
        <Box sx={{ display: "flex", alignItems: "center", gap: 1.5, minWidth: 0 }}>
          <Cover src={track.thumbnail} px={96} sx={{ width: 40, height: 40, fontSize: 20 }} />
          <Box sx={{ minWidth: 0 }}>
            <Typography variant="body2" noWrap color={active ? "primary" : "text.primary"}>{track.title}</Typography>
            {IS_MOBILE && (
              <Typography variant="caption" noWrap component="div" sx={subdued}><ArtistLink artist={track.artist} /></Typography>
            )}
          </Box>
        </Box>
        {!IS_MOBILE && (
          <Typography variant="body2" noWrap sx={subdued}><ArtistLink artist={track.artist} /></Typography>
        )}
        {!IS_MOBILE && showPlays && (
          <Typography variant="body2" noWrap sx={{ ...subdued, textAlign: "right", pr: 1 }}>{plays ?? ""}</Typography>
        )}
      </Box>

      <Box onClick={(e) => e.stopPropagation()} sx={{ width: ACTIONS_WIDTH, display: "flex", justifyContent: "flex-end" }}>
        {/* the buttons only show on hover, except a liked heart, and always where there is no hover */}
        <Tooltip title="Add to queue (Shift: play next)">
          <IconButton
            className="row-action" size="small" aria-label="Add to queue"
            onClick={(e) => player.enqueue([track], e.shiftKey ? "next" : "end")}
            sx={{ width: BUTTON, height: BUTTON, opacity: IS_MOBILE ? 1 : 0 }}
          >
            <PlaylistAddIcon fontSize="small" />
          </IconButton>
        </Tooltip>
        <Box className="row-action" sx={{ width: BUTTON, display: "flex", justifyContent: "center", opacity: liked || IS_MOBILE ? 1 : 0 }}>
          <LikeButton track={track} size="small" />
        </Box>
      </Box>
    </Box>
  );
}

/**
 * Tracks as numbered rows. The row of the playing track is tinted, and a click plays from that
 * row. `plays` adds a column of play counts, as display text, one entry per track.
 */
export default function TrackTable({
  tracks, plays, onPlay,
}: { tracks: Track[]; plays?: (string | null)[]; onPlay: (index: number) => void }) {
  const activeId = currentTrack(usePlayer())?.id;
  return (
    <Box sx={{ px: GUTTER }}>
      <Box sx={{ display: "flex", borderBottom: 1, borderColor: "divider", pb: 1, mb: 0.5 }}>
        <Box sx={{ flex: 1, display: "grid", gridTemplateColumns: columns(!!plays) }}>
          <Typography sx={{ ...columnHead, textAlign: "center" }}>#</Typography>
          <Typography sx={columnHead}>TITLE</Typography>
          {!IS_MOBILE && <Typography sx={columnHead}>ARTIST</Typography>}
          {!IS_MOBILE && plays && <Typography sx={{ ...columnHead, textAlign: "right", pr: 1 }}>PLAYS</Typography>}
        </Box>
        <Box sx={{ width: ACTIONS_WIDTH }} />
      </Box>
      {tracks.map((t, i) => (
        <Row key={`${t.id}:${i}`} track={t} index={i} active={t.id === activeId} plays={plays?.[i]} showPlays={!!plays} onPlay={() => onPlay(i)} />
      ))}
    </Box>
  );
}
