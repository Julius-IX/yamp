import { Box, Dialog, IconButton, Typography } from "@mui/material";
import CloseIcon from "@mui/icons-material/Close";
import ArtistLink from "@/browse/ArtistLink";
import { useBackGuard } from "@/router/useBackGuard";
import { IS_MOBILE } from "@/util/platform";
import { video, useVideo } from "./store";

/**
 * Plays a video through the provider's embedded player. A modal on desktop, a full screen page
 * on mobile with the player on top and the details below, like the provider's own app.
 */
export default function VideoModal() {
  const track = useVideo();
  useBackGuard(!!track && IS_MOBILE, video.close);

  return (
    <Dialog
      open={!!track}
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
            component="iframe"
            title={track.title}
            src={`https://www.youtube-nocookie.com/embed/${encodeURIComponent(track.id)}?autoplay=1&playsinline=1&rel=0`}
            allow="autoplay; encrypted-media; picture-in-picture; fullscreen"
            allowFullScreen
            referrerPolicy="strict-origin-when-cross-origin"
            sx={{ width: "100%", aspectRatio: "16 / 9", border: 0, bgcolor: "common.black" }}
          />
          <Box sx={{ p: 2, bgcolor: "background.paper" }}>
            <Typography variant="h6" sx={{ fontWeight: 700 }}>{track.title}</Typography>
            <Typography color="text.secondary">
              <ArtistLink artist={track.artist} onOpen={video.close} />
            </Typography>
          </Box>
        </Box>
      )}
    </Dialog>
  );
}
