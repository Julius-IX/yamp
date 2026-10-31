import { useEffect, useRef } from "react";
import { Box, Chip, Paper, Typography } from "@mui/material";

export type Now = {
  title: string;
  artist?: string;
  artwork?: string;
  src: string;
  origin: "stream" | "disk";
  detail?: string;
};

/** Client and format the provider put into a stream URL, which tells apart URLs that fail alike. */
function streamTag(src: string): string {
  try {
    const q = new URL(src).searchParams;
    return ` client=${q.get("c")} itag=${q.get("itag")}`;
  } catch {
    return "";
  }
}

/** Plays a stream URL or a downloaded file, the audio element does not care which. */
export default function Player({
  now,
  onError,
  onEnded,
}: {
  now: Now | null;
  onError: (message: string) => void;
  onEnded?: () => void;
}) {
  const audio = useRef<HTMLAudioElement>(null);

  // play() is called by hand because the autoplay attribute leaves a promise that cannot be caught.
  // AbortError is expected when the element is replaced mid-load and other failures reach the
  // error event, so only a blocked autoplay is reported here.
  useEffect(() => {
    audio.current?.play().catch((e: DOMException) => {
      if (e.name === "NotAllowedError") onError(`autoplay was blocked: ${e.message}`);
    });
  }, [now?.src, onError]);

  if (!now) {
    return (
      <Typography variant="body2" color="text.secondary">
        Nothing playing.
      </Typography>
    );
  }
  return (
    <Paper variant="outlined" sx={{ p: 1 }}>
      <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
        <Typography noWrap sx={{ flex: 1 }}>
          {now.title}
        </Typography>
        <Chip size="small" label={now.origin} />
      </Box>
      {now.detail && (
        <Typography variant="caption" color="text.secondary" noWrap component="div">
          {now.detail}
        </Typography>
      )}
      {/* keyed by source so switching tracks always loads a fresh element */}
      <audio
        ref={audio}
        key={now.src}
        src={now.src}
        controls
        onEnded={onEnded}
        style={{ width: "100%" }}
        onError={(e) => {
          const a = e.currentTarget;
          // code 4 alone does not say whether the source or the decoder failed, the position tells
          // a failure at the start from one mid-playback
          onError(
            `playback of ${now.title} (${now.origin}): code=${a.error?.code} "${a.error?.message ?? ""}" ` +
            `at=${a.currentTime.toFixed(1)}s mp4=${a.canPlayType("audio/mp4")} ` +
            `webm=${a.canPlayType("audio/webm; codecs=opus")}${now.origin === "stream" ? streamTag(now.src) : ""}`,
          );
        }}
      />
    </Paper>
  );
}
