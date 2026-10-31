import { Box, Button, Chip, IconButton, Tooltip, Typography } from "@mui/material";
import CloseIcon from "@mui/icons-material/Close";
import PlayArrowIcon from "@mui/icons-material/PlayArrow";
import SkipNextIcon from "@mui/icons-material/SkipNext";
import { MAX_QUEUED, type QueuedTrack, type QueueState } from "./useStreamQueue";

type Props = {
  entries: QueuedTrack[];
  onPlay: (id: string) => void;
  onRemove: (id: string) => void;
  onNext: () => void;
  onClear: () => void;
};

const COLORS: Record<QueueState, "default" | "primary" | "success" | "error"> = {
  pending: "default",
  resolving: "primary",
  ready: "success",
  failed: "error",
};

/** The in-memory playback queue. Nothing here touches the disk, entries only hold stream URLs. */
export default function PlayQueuePanel({ entries, onPlay, onRemove, onNext, onClear }: Props) {
  return (
    <Box>
      <Box sx={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
        <Typography variant="subtitle2">
          Play queue ({entries.length}/{MAX_QUEUED})
        </Typography>
        <Box>
          <Tooltip title="Play the next track">
            <span>
              <IconButton size="small" onClick={onNext} disabled={entries.length === 0}>
                <SkipNextIcon fontSize="small" />
              </IconButton>
            </span>
          </Tooltip>
          <Button size="small" onClick={onClear} disabled={entries.length === 0}>
            Clear
          </Button>
        </Box>
      </Box>
      {entries.length === 0 && (
        <Typography variant="body2" color="text.secondary">
          Empty.
        </Typography>
      )}
      {entries.map((e) => (
        <Box key={e.track.id} sx={{ py: 0.5 }}>
          <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
            <Typography noWrap sx={{ flex: 1 }}>
              {e.track.title}
            </Typography>
            <Chip size="small" color={COLORS[e.state]} label={e.state} />
            <Tooltip title="Play now">
              <IconButton size="small" onClick={() => onPlay(e.track.id)}>
                <PlayArrowIcon fontSize="small" />
              </IconButton>
            </Tooltip>
            <Tooltip title="Remove">
              <IconButton size="small" onClick={() => onRemove(e.track.id)}>
                <CloseIcon fontSize="small" />
              </IconButton>
            </Tooltip>
          </Box>
          {e.error && (
            <Typography variant="caption" color="error">
              {e.error}
            </Typography>
          )}
        </Box>
      ))}
    </Box>
  );
}
