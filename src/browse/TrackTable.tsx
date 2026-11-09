import { Box, IconButton, Tooltip, Typography } from "@mui/material";
import CloseIcon from "@mui/icons-material/Close";
import PlayArrowIcon from "@mui/icons-material/PlayArrow";
import PlaylistAddIcon from "@mui/icons-material/PlaylistAdd";
import { useIsLiked } from "@/lib/likes";
import { useDownloadStatus } from "@/lib/local";
import type { Track } from "@/lib/types";
import Cover from "@/player/Cover";
import LikeButton from "@/player/LikeButton";
import { currentTrack, player, usePlayer } from "@/player/store";
import { IS_MOBILE } from "@/util/platform";
import AddToPlaylist from "@/library/AddToPlaylist";
import DownloadButton from "@/library/DownloadButton";
import ArtistLink from "./ArtistLink";
import { columnHead, faint, GUTTER, radius, subdued } from "@/themes/ui";

// The header and the rows share these so the columns line up. The buttons sit after them.
const columns = (plays: boolean) =>
  IS_MOBILE ? "32px minmax(0, 1fr)" : `40px minmax(0, 1.4fr) minmax(0, 1fr)${plays ? " 96px" : ""}`;
const BUTTON = 40;
/** The queue, playlist, download and like buttons, and the remove button when the table can remove. */
const actionsWidth = (removable: boolean) => BUTTON * (removable ? 5 : 4);

/**
 * One slot of the action buttons. Every button sits in the same fixed box, centered on both axes,
 * so they line up whatever their own size is. The tooltip wraps a button in an inline span, which
 * would otherwise sit on the text baseline and look higher than a bare button.
 */
const cell = (shown: boolean) => ({
  width: BUTTON, height: BUTTON, display: "flex", alignItems: "center", justifyContent: "center",
  opacity: shown || IS_MOBILE ? 1 : 0,
  "& > span": { display: "flex" },
});

type RowProps = {
  track: Track; index: number; active: boolean; plays?: string | null; showPlays: boolean; onPlay: () => void;
  onRemove?: () => void;
};

/**
 * The row is a div that plays when clicked, not a button. The artist and the actions inside are
 * controls of their own, and a button cannot hold another one.
 */
function Row({ track, index, active, plays, showPlays, onPlay, onRemove }: RowProps) {
  const liked = useIsLiked(track.id);
  const download = useDownloadStatus(track.id);
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

      <Box onClick={(e) => e.stopPropagation()} sx={{ width: actionsWidth(!!onRemove), display: "flex", alignItems: "center", justifyContent: "flex-end" }}>
        {/* the buttons only show on hover, except a liked heart or a download, and always where there is no hover */}
        <Box className="row-action" sx={cell(true)}>
          <Tooltip title="Add to queue (Shift: play next)">
            <IconButton
              size="small" aria-label="Add to queue"
              onClick={(e) => player.enqueue([track], e.shiftKey ? "next" : "end")}
            >
              <PlaylistAddIcon fontSize="small" />
            </IconButton>
          </Tooltip>
        </Box>
        <Box className="row-action" sx={cell(true)}>
          <AddToPlaylist track={track} />
        </Box>
        {/* a download that is under way or done stays visible, like a liked heart */}
        <Box className="row-action" sx={cell(download !== "none")}>
          <DownloadButton track={track} />
        </Box>
        <Box className="row-action" sx={cell(liked)}>
          <LikeButton track={track} size="small" />
        </Box>
        {onRemove && (
          <Box className="row-action" sx={cell(true)}>
            <Tooltip title="Remove from playlist">
              <IconButton size="small" aria-label="Remove from playlist" onClick={onRemove}>
                <CloseIcon fontSize="small" />
              </IconButton>
            </Tooltip>
          </Box>
        )}
      </Box>
    </Box>
  );
}

/**
 * Tracks as numbered rows. The row of the playing track is tinted, and a click plays from that
 * row. `plays` adds a column of play counts, as display text, one entry per track. `onRemove`
 * adds a button that takes a row out of the list the table shows, for a playlist of the library.
 */
export default function TrackTable({
  tracks, plays, onPlay, onRemove,
}: { tracks: Track[]; plays?: (string | null)[]; onPlay: (index: number) => void; onRemove?: (index: number) => void }) {
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
        <Box sx={{ width: actionsWidth(!!onRemove) }} />
      </Box>
      {tracks.map((t, i) => (
        <Row key={`${t.id}:${i}`} track={t} index={i} active={t.id === activeId} plays={plays?.[i]} showPlays={!!plays} onPlay={() => onPlay(i)} onRemove={onRemove && (() => onRemove(i))} />
      ))}
    </Box>
  );
}
