import { useEffect, useMemo, useRef } from "react";
import { Box, Dialog, IconButton, Typography } from "@mui/material";
import CloseIcon from "@mui/icons-material/Close";
import ArtistLink from "@/browse/ArtistLink";
import { isLocalFile } from "@/lib/local";
import { currentTrack, getPlayer, getTime, onSeek, usePlayer } from "@/player/store";
import SeekBar from "@/player/SeekBar";
import TransportControls from "@/player/TransportControls";
import { isRadio } from "@/radio/station";
import { useBackGuard } from "@/router/useBackGuard";
import { IS_MOBILE } from "@/util/platform";
import { video, useVideoOpen } from "./store";

const EMBED = "https://www.youtube-nocookie.com";

/** How far the picture may drift from the audio before it is moved back, in seconds. */
const MAX_DRIFT = 2;

/**
 * The picture of the song that plays, through the provider's embedded player. The audio comes
 * from the app's player and the embed is muted, so it only follows it. It starts where the song
 * is, pauses and resumes with it and is moved back when it falls behind. It shows whichever song
 * plays, so skipping to the next one changes the picture as well.
 *
 * A modal on desktop, a full screen page on mobile with the picture on top and the controls
 * below, like the provider's own app.
 */
export default function VideoModal() {
  const wanted = useVideoOpen();
  const state = usePlayer();
  const track = currentTrack(state);
  // stations and files on the device have no picture to show
  const watchable = !!track && !isRadio(track.id) && !isLocalFile(track.id);
  const open = wanted && watchable;
  useBackGuard(open && IS_MOBILE, video.close);

  const frame = useRef<HTMLIFrameElement>(null);
  const command = (func: string, args: unknown[] = []) =>
    frame.current?.contentWindow?.postMessage(JSON.stringify({ event: "command", func, args }), "*");

  // the embed starts at the position the song has when it appears, later moves are commands
  const id = track?.id;
  const src = useMemo(() => {
    if (!id) return "";
    const start = Math.floor(getTime().position);
    // controls are off since the player of the app is the one that controls the song
    return `${EMBED}/embed/${encodeURIComponent(id)}?autoplay=1&mute=1&controls=0&disablekb=1&playsinline=1&rel=0&enablejsapi=1&start=${start}`;
  }, [id]);

  useEffect(() => command(state.playing ? "playVideo" : "pauseVideo"), [state.playing, id]);
  useEffect(() => onSeek(() => command("seekTo", [getTime().position, true])), []);

  // the embed reports its position once it is listening, which is how a drift is noticed
  useEffect(() => {
    const onMessage = (e: MessageEvent) => {
      if (e.source !== frame.current?.contentWindow || typeof e.data !== "string") return;
      try {
        const data = JSON.parse(e.data);
        const at = data?.info?.currentTime;
        if (data.event !== "infoDelivery" || typeof at !== "number" || !getPlayer().playing) return;
        if (Math.abs(at - getTime().position) > MAX_DRIFT) command("seekTo", [getTime().position, true]);
      } catch {
        // messages of the embed that are not JSON are not about the position
      }
    };
    window.addEventListener("message", onMessage);
    return () => window.removeEventListener("message", onMessage);
  }, []);

  const listen = () => {
    frame.current?.contentWindow?.postMessage(JSON.stringify({ event: "listening", id: 1, channel: "widget" }), "*");
    command(getPlayer().playing ? "playVideo" : "pauseVideo");
  };

  return (
    <Dialog
      open={open}
      onClose={video.close}
      fullScreen={IS_MOBILE}
      fullWidth
      maxWidth="md"
      slotProps={{ paper: { sx: { bgcolor: IS_MOBILE ? "background.default" : "common.black", backgroundImage: "none" } } }}
    >
      {track && (
        <Box sx={{ display: "flex", flexDirection: "column", pt: IS_MOBILE ? "env(safe-area-inset-top)" : 0 }}>
          {IS_MOBILE && (
            <Box sx={{ display: "flex", justifyContent: "flex-end", p: 0.5 }}>
              <IconButton aria-label="Close video" onClick={video.close}>
                <CloseIcon />
              </IconButton>
            </Box>
          )}
          {/* the embed can refuse to play (error 153) when the app origin sends no referrer */}
          <Box
            key={track.id}
            ref={frame}
            component="iframe"
            title={track.title}
            src={src}
            onLoad={listen}
            allow="autoplay; encrypted-media; picture-in-picture; fullscreen"
            referrerPolicy="strict-origin-when-cross-origin"
            sx={{ width: "100%", aspectRatio: "16 / 9", border: 0, bgcolor: "common.black", pointerEvents: "none" }}
          />
          <Box sx={{ p: 2, bgcolor: "background.paper", display: "flex", flexDirection: "column", gap: 1 }}>
            <Box>
              <Typography variant="h6" sx={{ fontWeight: 700 }}>{track.title}</Typography>
              <Typography color="text.secondary">
                <ArtistLink artist={track.artist} onOpen={video.close} />
              </Typography>
            </Box>
            <SeekBar />
            <TransportControls />
          </Box>
        </Box>
      )}
    </Dialog>
  );
}
