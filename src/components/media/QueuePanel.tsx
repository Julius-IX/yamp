import { Box, Button, Chip, IconButton, LinearProgress, Tooltip, Typography } from "@mui/material";
import CloseIcon from "@mui/icons-material/Close";
import PlayArrowIcon from "@mui/icons-material/PlayArrow";
import type { DownloadJob } from "../../lib/types";

type Props = {
  jobs: DownloadJob[];
  titleOf: (id: string) => string;
  onCancel: (id: string) => void;
  onPlay: (job: DownloadJob) => void;
  onClear: () => void;
};

const COLORS = {
  queued: "default",
  running: "primary",
  done: "success",
  failed: "error",
  cancelled: "default",
} as const;

/** The on-disk download queue, separate from the in-memory play queue. A job can be cancelled until it starts and played once it is on disk. */
export default function QueuePanel({ jobs, titleOf, onCancel, onPlay, onClear }: Props) {
  return (
    <Box>
      <Box sx={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
        <Typography variant="subtitle2">Downloads (on disk)</Typography>
        <Button size="small" onClick={onClear} disabled={jobs.every((j) => j.state === "queued" || j.state === "running")}>
          Clear finished
        </Button>
      </Box>
      {jobs.length === 0 && (
        <Typography variant="body2" color="text.secondary">
          Empty.
        </Typography>
      )}
      {jobs.map((j) => (
        <Box key={j.id} sx={{ py: 0.5 }}>
          <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
            <Typography noWrap sx={{ flex: 1 }}>
              {titleOf(j.id)}
            </Typography>
            <Chip size="small" color={COLORS[j.state]} label={j.state} />
            {j.state === "queued" && (
              <Tooltip title="Cancel">
                <IconButton size="small" onClick={() => onCancel(j.id)}>
                  <CloseIcon fontSize="small" />
                </IconButton>
              </Tooltip>
            )}
            {j.state === "done" && (
              <Tooltip title="Play from disk">
                <IconButton size="small" onClick={() => onPlay(j)}>
                  <PlayArrowIcon fontSize="small" />
                </IconButton>
              </Tooltip>
            )}
          </Box>
          {j.state === "running" && <LinearProgress variant="determinate" value={j.percent} />}
          {j.error && (
            <Typography variant="caption" color="error">
              {j.error}
            </Typography>
          )}
        </Box>
      ))}
    </Box>
  );
}
