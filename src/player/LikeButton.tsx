import { IconButton, Tooltip } from "@mui/material";
import FavoriteIcon from "@mui/icons-material/Favorite";
import FavoriteBorderIcon from "@mui/icons-material/FavoriteBorder";
import { toggleLike, useIsLiked } from "@/lib/likes";
import type { Track } from "@/lib/types";

export default function LikeButton({ track, size = "medium" }: { track?: Track; size?: "small" | "medium" }) {
  const liked = useIsLiked(track?.id);
  const label = liked ? "Remove from liked songs" : "Add to liked songs";
  return (
    <Tooltip title={track ? label : ""}>
      <span>
        <IconButton
          size={size}
          aria-label={label}
          disabled={!track}
          color={liked ? "primary" : "default"}
          onClick={() => track && toggleLike(track)}
        >
          {liked ? <FavoriteIcon fontSize="inherit" /> : <FavoriteBorderIcon fontSize="inherit" />}
        </IconButton>
      </span>
    </Tooltip>
  );
}
