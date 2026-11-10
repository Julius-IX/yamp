import { Box } from "@mui/material";
import { IS_MOBILE } from "@/util/platform";
import ErrorNotice from "./ErrorNotice";
import UpdateNotice from "./UpdateNotice";

/**
 * Notices that float above the player. The stack is a zero height row placed right before the
 * player, and its content is anchored to the bottom of that row, so it sits on top of the player
 * bar on desktop and on top of the mini player and the bottom navigation on mobile. With nothing
 * playing it simply sits on top of whatever is at the bottom. The row lets clicks through, only
 * the notices themselves take them.
 */
export default function NoticeStack() {
  return (
    <Box sx={{ position: "relative", height: 0, flexShrink: 0, zIndex: (t) => t.zIndex.snackbar }}>
      <Box
        sx={{
          position: "absolute", left: 0, right: 0, bottom: 0, p: 1.5,
          display: "flex", flexDirection: "column", gap: 1, pointerEvents: "none",
          alignItems: IS_MOBILE ? "stretch" : "flex-end",
          "& > *": { pointerEvents: "auto", width: IS_MOBILE ? "auto" : 400, maxWidth: "100%" },
        }}
      >
        <ErrorNotice />
        <UpdateNotice />
      </Box>
    </Box>
  );
}
