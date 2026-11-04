import { forwardRef, ReactElement, Ref, useEffect, useState } from "react";
import { Box, Dialog, IconButton, Slide, Typography } from "@mui/material";
import { TransitionProps } from "@mui/material/transitions";
import ExpandMoreIcon from "@mui/icons-material/ExpandMore";
import QueueMusicIcon from "@mui/icons-material/QueueMusic";
import { useBackGuard } from "@/router/useBackGuard";
import Cover from "./Cover";
import LikeButton from "./LikeButton";
import QueuePanel from "./QueuePanel";
import SeekBar from "./SeekBar";
import { currentTrack, usePlayer } from "./store";
import TransportControls from "./TransportControls";

const SlideUp = forwardRef(function SlideUp(props: TransitionProps & { children: ReactElement }, ref: Ref<unknown>) {
  return <Slide direction="up" ref={ref} {...props} />;
});

/** Mobile only. The full screen player that the mini player opens, with the queue as a second view. */
export default function NowPlayingSheet({ open, onClose }: { open: boolean; onClose: () => void }) {
  const state = usePlayer();
  const track = currentTrack(state);
  const [queue, setQueue] = useState(false);

  // one guard per view, so that going back from the queue leaves the sheet open and still guarded
  useBackGuard(open && !queue, onClose);
  useBackGuard(open && queue, () => setQueue(false));

  // the queue view is not kept for the next time the sheet opens
  useEffect(() => {
    if (!open) setQueue(false);
  }, [open]);

  return (
    <Dialog fullScreen open={open} onClose={onClose} slots={{ transition: SlideUp }}>
      <Box
        sx={{
          display: "flex", flexDirection: "column", height: "100%",
          pt: "env(safe-area-inset-top)", pb: "env(safe-area-inset-bottom)",
        }}
      >
        {queue ? (
          <QueuePanel onClose={() => setQueue(false)} />
        ) : (
          <>
            <Box sx={{ display: "flex", alignItems: "center", p: 1 }}>
              <IconButton aria-label="Close player" onClick={onClose}>
                <ExpandMoreIcon />
              </IconButton>
              <Typography variant="subtitle2" sx={{ flex: 1, textAlign: "center" }}>Now playing</Typography>
              <IconButton aria-label="Queue" onClick={() => setQueue(true)}>
                <QueueMusicIcon />
              </IconButton>
            </Box>

            <Box sx={{ flex: 1, minHeight: 0, display: "flex", alignItems: "center", justifyContent: "center", px: 3 }}>
              <Cover
                src={track?.thumbnail}
                sx={{ width: "100%", maxWidth: 360, height: "auto", aspectRatio: "1", borderRadius: 2, fontSize: 96 }}
              />
            </Box>

            <Box sx={{ display: "flex", flexDirection: "column", gap: 1, px: 3, pb: 3 }}>
              <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
                <Box sx={{ flex: 1, minWidth: 0 }}>
                  <Typography variant="h6" noWrap sx={{ fontWeight: 700 }}>{track?.title}</Typography>
                  <Typography color="text.secondary" noWrap>{track?.artist}</Typography>
                </Box>
                <LikeButton track={track} />
              </Box>
              <SeekBar />
              <TransportControls large />
            </Box>
          </>
        )}
      </Box>
    </Dialog>
  );
}
