import type { MouseEvent } from "react";
import { Box, ButtonBase, Skeleton, Typography } from "@mui/material";
import MusicNoteIcon from "@mui/icons-material/MusicNote";
import PlayArrowIcon from "@mui/icons-material/PlayArrow";
import PlaylistAddIcon from "@mui/icons-material/PlaylistAdd";
import Cover from "@/player/Cover";
import { IS_MOBILE } from "@/util/platform";
import { accentFill, faint, radius, scrim, subdued, surface } from "@/themes/ui";

/** Square for tracks, albums and playlists, round for artists, 16:9 for videos. */
export type CardVariant = "square" | "round" | "wide";

const WIDTH: Record<CardVariant, number> = IS_MOBILE
  ? { square: 140, round: 120, wide: 200 }
  : { square: 180, round: 156, wide: 240 };

const COVER: Record<CardVariant, object> = {
  square: { aspectRatio: "1" },
  round: { aspectRatio: "1", borderRadius: "50%" },
  wide: { aspectRatio: "16 / 9" },
};

const shell = (variant: CardVariant) =>
  ({
    display: "block", flexShrink: 0, width: WIDTH[variant], p: 1.25, borderRadius: radius.card, textAlign: "left",
    scrollSnapAlign: "start",
    ...(variant === "round" ? { textAlign: "center", border: 1, borderColor: "transparent" } : surface),
  }) as const;

type Props = {
  thumbnail?: string | null;
  title: string;
  subtitle?: string;
  variant?: CardVariant;
  onClick: () => void;
  /** Plays without opening. Without it a click on the overlay is a click on the card. */
  onPlay?: () => void;
  /** Adds to the end of the queue. Shown on hover, or always where there is no hover. */
  onQueue?: () => void;
};

/**
 * A cover with a title and a line below. The overlay is a hint that the card plays, so on a
 * touch screen without hover it is left out. Artists have no overlay, they only open.
 */
export default function MediaCard({ thumbnail, title, subtitle, variant = "square", onClick, onPlay, onQueue }: Props) {
  return (
    <ButtonBase
      onClick={onClick}
      sx={{
        ...shell(variant), transition: "background-color 0.2s, border-color 0.2s",
        "&:hover": { bgcolor: "action.hover", ...(variant !== "round" && { borderColor: "text.disabled" }) },
        "&:hover .card-play": { opacity: 1, transform: "none" },
      }}
    >
      <Box sx={{ position: "relative" }}>
        <Cover
          src={thumbnail}
          px={variant === "wide" ? 480 : 320}
          sx={{ width: "100%", fontSize: 40, ...COVER[variant] }}
        />
        {variant !== "round" && (
          <Box
            className="card-play"
            component="span"
            aria-hidden
            onClick={
              onPlay &&
              ((e: MouseEvent) => {
                e.stopPropagation();
                onPlay();
              })
            }
            sx={{
              position: "absolute", right: 8, bottom: 8, width: 36, height: 36, borderRadius: "50%",
              display: "flex", alignItems: "center", justifyContent: "center",
              ...accentFill,
              opacity: 0, transform: "translateY(4px)", transition: "opacity 0.2s, transform 0.2s",
              "@media (hover: none)": { display: "none" },
            }}
          >
            <PlayArrowIcon />
          </Box>
        )}
        {onQueue && variant !== "round" && (
          <Box
            className="card-play"
            component="span"
            aria-hidden
            title="Add to queue"
            onClick={(e: MouseEvent) => {
              e.stopPropagation();
              onQueue();
            }}
            sx={{
              position: "absolute", right: 8, top: 8, width: 30, height: 30, borderRadius: "50%",
              display: "flex", alignItems: "center", justifyContent: "center",
              ...scrim,
              opacity: 0, transform: "translateY(-4px)", transition: "opacity 0.2s, transform 0.2s",
              // no hover on a touch screen, so it is there all the time
              "@media (hover: none)": { opacity: 1, transform: "none" },
            }}
          >
            <PlaylistAddIcon fontSize="small" />
          </Box>
        )}
      </Box>
      <Typography variant="body2" noWrap sx={{ mt: 1.25, fontWeight: 600 }}>{title}</Typography>
      {subtitle && (
        <Typography
          variant="caption" noWrap component="div"
          sx={{ ...subdued, ...(variant === "round" && { textTransform: "uppercase", letterSpacing: 1, fontSize: 10 }) }}
        >
          {subtitle}
        </Typography>
      )}
    </ButtonBase>
  );
}

/** Animated, for a shelf that is still loading. */
export function CardSkeleton({ variant = "square" }: { variant?: CardVariant }) {
  return (
    <Box sx={shell(variant)}>
      <Skeleton
        variant={variant === "round" ? "circular" : "rounded"}
        sx={{ width: "100%", height: "auto", ...COVER[variant] }}
      />
      <Skeleton width="80%" sx={{ mt: 1.25, mx: variant === "round" ? "auto" : 0 }} />
      <Skeleton width="50%" sx={{ mx: variant === "round" ? "auto" : 0 }} />
    </Box>
  );
}

/** Still, for a shelf with nothing in it yet. It keeps the row and its size in place. */
export function CardPlaceholder({ variant = "square" }: { variant?: CardVariant }) {
  const bar = (width: string) => ({ height: 10, width, borderRadius: 1, bgcolor: "action.hover", mx: variant === "round" ? "auto" : 0 });
  return (
    <Box sx={{ ...shell(variant), ...(variant !== "round" && { borderStyle: "dashed" }) }} aria-hidden>
      <Box
        sx={{
          width: "100%", borderRadius: 1, bgcolor: "action.hover", ...faint,
          display: "flex", alignItems: "center", justifyContent: "center", fontSize: 40, ...COVER[variant],
        }}
      >
        <MusicNoteIcon fontSize="inherit" />
      </Box>
      <Box sx={{ ...bar("75%"), mt: 1.75 }} />
      <Box sx={{ ...bar("45%"), mt: 1 }} />
    </Box>
  );
}
