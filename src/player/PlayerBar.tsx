import { useState } from "react";
import { Box, Drawer, IconButton, Paper, Tooltip, Typography } from "@mui/material";
import QueueMusicIcon from "@mui/icons-material/QueueMusic";
import Cover from "./Cover";
import LikeButton from "./LikeButton";
import QueuePanel from "./QueuePanel";
import SeekBar from "./SeekBar";
import { currentTrack, usePlayer } from "./store";
import TransportControls from "./TransportControls";
import VolumeControl from "./VolumeControl";

const HEIGHT = 88;
const QUEUE_WIDTH = 340;

/**
 * The desktop player. It sits under the sidebar and the page and spans the window, in three
 * columns: the track, the controls and the extras (queue and volume).
 */
export default function PlayerBar() {
  const state = usePlayer();
  const track = currentTrack(state);
  const [queueOpen, setQueueOpen] = useState(false);

  return (
    <>
      <Paper
        square
        elevation={0}
        sx={{
          display: "grid", gridTemplateColumns: "1fr minmax(300px, 2fr) 1fr", alignItems: "center", gap: 2,
          flexShrink: 0, height: HEIGHT, px: 2, borderTop: 1, borderColor: "divider",
          // above the queue drawer, which is fixed and ends where the bar starts
          position: "relative", zIndex: (t) => t.zIndex.drawer + 1,
        }}
      >
        <Box sx={{ display: "flex", alignItems: "center", gap: 1.5, minWidth: 0 }}>
          <Cover src={track?.thumbnail} sx={{ width: 56, height: 56, flexShrink: 0, fontSize: 28 }} />
          <Box sx={{ minWidth: 0 }}>
            <Typography noWrap sx={{ fontWeight: 600 }}>{track?.title ?? "Pick something to listen to"}</Typography>
            <Typography variant="body2" color="text.secondary" noWrap>{track?.artist ?? "Search, or open your library"}</Typography>
          </Box>
          <LikeButton track={track} />
        </Box>

        <Box sx={{ display: "flex", flexDirection: "column", alignItems: "center", minWidth: 0 }}>
          <TransportControls />
          <SeekBar />
        </Box>

        <Box sx={{ display: "flex", alignItems: "center", justifyContent: "flex-end", gap: 0.5 }}>
          <Tooltip title="Queue">
            <IconButton
              size="small"
              aria-label="Queue"
              color={queueOpen ? "primary" : "default"}
              onClick={() => setQueueOpen((o) => !o)}
            >
              <QueueMusicIcon fontSize="small" />
            </IconButton>
          </Tooltip>
          <VolumeControl />
        </Box>
      </Paper>

      {/* persistent, so the rest of the app stays usable while the queue is open */}
      <Drawer
        variant="persistent"
        anchor="right"
        open={queueOpen}
        slotProps={{
          paper: { sx: { width: QUEUE_WIDTH, height: `calc(100% - ${HEIGHT}px)`, borderLeft: 1, borderColor: "divider" } },
        }}
      >
        <QueuePanel onClose={() => setQueueOpen(false)} />
      </Drawer>
    </>
  );
}
