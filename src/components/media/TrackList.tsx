import { Avatar, Box, IconButton, Stack, Tooltip, Typography } from "@mui/material";
import DownloadIcon from "@mui/icons-material/Download";
import InfoOutlinedIcon from "@mui/icons-material/InfoOutlined";
import PlaylistAddIcon from "@mui/icons-material/PlaylistAdd";
import PlayArrowIcon from "@mui/icons-material/PlayArrow";
import RadioIcon from "@mui/icons-material/Radio";
import type { Track } from "../../lib/types";
import { duration } from "./format";

type Props = {
  tracks: Track[];
  onPlay: (track: Track) => void;
  onEnqueue: (track: Track) => void;
  onDownload: (track: Track) => void;
  onInfo: (track: Track) => void;
  onRelated: (track: Track) => void;
};

/** Tracks from any source (search, playlist, related) with the actions each one supports. */
export default function TrackList({ tracks, onPlay, onEnqueue, onDownload, onInfo, onRelated }: Props) {
  if (tracks.length === 0) {
    return (
      <Typography variant="body2" color="text.secondary">
        Nothing here yet.
      </Typography>
    );
  }
  return (
    <Stack>
      {tracks.map((t) => (
        <Box key={t.id} sx={{ display: "flex", alignItems: "center", gap: 1, py: 0.5 }}>
          <Avatar variant="rounded" src={t.thumbnail ?? undefined} sx={{ width: 40, height: 40 }} />
          <Box sx={{ flex: 1, minWidth: 0 }}>
            <Typography noWrap>{t.title}</Typography>
            <Typography variant="caption" color="text.secondary" noWrap component="div">
              {t.artist} · {duration(t.durationSecs)}
            </Typography>
          </Box>
          <Tooltip title="Play stream">
            <IconButton size="small" onClick={() => onPlay(t)}>
              <PlayArrowIcon fontSize="small" />
            </IconButton>
          </Tooltip>
          <Tooltip title="Add to play queue">
            <IconButton size="small" onClick={() => onEnqueue(t)}>
              <PlaylistAddIcon fontSize="small" />
            </IconButton>
          </Tooltip>
          <Tooltip title="Download to disk">
            <IconButton size="small" onClick={() => onDownload(t)}>
              <DownloadIcon fontSize="small" />
            </IconButton>
          </Tooltip>
          <Tooltip title="Related tracks">
            <IconButton size="small" onClick={() => onRelated(t)}>
              <RadioIcon fontSize="small" />
            </IconButton>
          </Tooltip>
          <Tooltip title="Full metadata">
            <IconButton size="small" onClick={() => onInfo(t)}>
              <InfoOutlinedIcon fontSize="small" />
            </IconButton>
          </Tooltip>
        </Box>
      ))}
    </Stack>
  );
}
