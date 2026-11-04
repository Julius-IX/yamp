import { Box, ButtonBase, Button, FormControlLabel, IconButton, Switch, Tooltip, Typography } from "@mui/material";
import CloseIcon from "@mui/icons-material/Close";
import { isBoolean, useSetting } from "@/lib/settings";
import type { Track } from "@/lib/types";
import Cover from "./Cover";
import { currentTrack, player, usePlayer } from "./store";

function Row({ track, active = false, onClick, onRemove }: {
  track: Track;
  active?: boolean;
  onClick?: () => void;
  onRemove?: () => void;
}) {
  return (
    <ButtonBase
      component="div"
      onClick={onClick}
      disabled={!onClick}
      sx={{
        display: "flex", alignItems: "center", gap: 1.5, width: "100%", px: 2, py: 0.75, textAlign: "left",
        justifyContent: "flex-start", "&:hover .queue-remove": { opacity: 1 },
        // a disabled ButtonBase would otherwise drop pointer events on the remove button as well
        "&.Mui-disabled": { opacity: 1, pointerEvents: "auto" },
      }}
    >
      <Cover src={track.thumbnail} sx={{ width: 40, height: 40 }} />
      <Box sx={{ flex: 1, minWidth: 0 }}>
        <Typography noWrap color={active ? "primary" : "text.primary"} sx={{ fontWeight: active ? 600 : 400 }}>
          {track.title}
        </Typography>
        <Typography variant="caption" color="text.secondary" noWrap component="div">
          {track.artist}
        </Typography>
      </Box>
      {onRemove && (
        <IconButton
          className="queue-remove"
          size="small"
          aria-label={`Remove ${track.title}`}
          onClick={(e) => { e.stopPropagation(); onRemove(); }}
          // always shown without hover, since a touch screen has none
          sx={{ opacity: 0, "@media (hover: none)": { opacity: 1 } }}
        >
          <CloseIcon fontSize="small" />
        </IconButton>
      )}
    </ButtonBase>
  );
}

/** What is playing and what comes after it. Tracks that already played are not listed, previous goes back to them. */
export default function QueuePanel({ onClose }: { onClose: () => void }) {
  const state = usePlayer();
  const [autoplay, setAutoplay] = useSetting("autoplay", true, isBoolean);
  const now = currentTrack(state);
  const upcoming = state.queue.slice(state.index + 1);

  return (
    <Box sx={{ display: "flex", flexDirection: "column", height: "100%", minHeight: 0 }}>
      <Box sx={{ display: "flex", alignItems: "center", px: 2, pt: 1.5, pb: 0.5 }}>
        <Typography variant="subtitle1" sx={{ flex: 1, fontWeight: 700 }}>Queue</Typography>
        <IconButton size="small" aria-label="Close queue" onClick={onClose}>
          <CloseIcon fontSize="small" />
        </IconButton>
      </Box>
      <Box sx={{ display: "flex", alignItems: "center", justifyContent: "space-between", px: 2, pb: 1 }}>
        <Tooltip title="Adds related tracks when the queue runs out">
          <FormControlLabel
            label="Autoplay"
            control={<Switch size="small" checked={autoplay} onChange={(_, on) => setAutoplay(on)} />}
          />
        </Tooltip>
        <Button size="small" disabled={state.queue.length === 0} onClick={player.clear}>Clear</Button>
      </Box>

      <Box sx={{ flex: 1, minHeight: 0, overflowY: "auto", scrollbarWidth: "none", "&::-webkit-scrollbar": { display: "none" } }}>
        {!now && <Typography variant="body2" color="text.secondary" sx={{ px: 2 }}>The queue is empty.</Typography>}
        {now && (
          <>
            <Typography variant="overline" color="text.secondary" sx={{ px: 2 }}>Now playing</Typography>
            <Row track={now} active />
            <Typography variant="overline" color="text.secondary" sx={{ px: 2, display: "block", mt: 1 }}>
              Next up
            </Typography>
            {upcoming.length === 0 && (
              <Typography variant="body2" color="text.secondary" sx={{ px: 2 }}>
                {autoplay ? "Related tracks are added when this one ends." : "Nothing after this."}
              </Typography>
            )}
            {upcoming.map((t, i) => {
              const at = state.index + 1 + i;
              return <Row key={`${t.id}-${at}`} track={t} onClick={() => player.jumpTo(at)} onRemove={() => player.removeAt(at)} />;
            })}
          </>
        )}
      </Box>
    </Box>
  );
}
