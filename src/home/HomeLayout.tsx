import { Box, IconButton } from "@mui/material";
import ArrowBackIcon from "@mui/icons-material/ArrowBack";
import { Outlet, useLocation, useNavigate } from "react-router-dom";
import { floating, GUTTER } from "@/themes/ui";
import SearchBar from "./SearchBar";

const BACK_SIZE = 36;
/** The back button and the gap after it, which the search bar keeps clear of. */
const BACK_INSET = BACK_SIZE + 12;

/**
 * The search bar and the back button float over every screen of the tab and stay in place
 * while the content scrolls. They sit in a zero height sticky box, so they take no room and a
 * screen like an artist page can run its banner right under them. Being above the outlet keeps
 * the field mounted, and focused, while the screen below it changes.
 *
 * Back steps through the tab's own history, so a screen can be left without rebuilding the
 * route to the one before it. The first entry has the key "default" and nothing to go back to.
 */
export default function HomeLayout() {
  const navigate = useNavigate();
  const canGoBack = useLocation().key !== "default";

  return (
    <Box sx={{ position: "relative", minHeight: "100%", minWidth: 0 }}>
      <Box sx={{ position: "sticky", top: 0, zIndex: 10, height: 0 }}>
        <Box sx={{ position: "relative", pt: 3, px: GUTTER }}>
          {canGoBack && (
            <IconButton
              aria-label="Back"
              onClick={() => navigate(-1)}
              sx={{ position: "absolute", left: (t) => t.spacing(GUTTER), top: (t) => t.spacing(3.25), width: BACK_SIZE, height: BACK_SIZE, ...floating }}
            >
              <ArrowBackIcon fontSize="small" />
            </IconButton>
          )}
          <SearchBar leftInset={canGoBack ? BACK_INSET : 0} />
        </Box>
      </Box>
      <Outlet />
    </Box>
  );
}
