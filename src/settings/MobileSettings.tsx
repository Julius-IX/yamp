import { Box, IconButton, ListItemButton, Typography } from "@mui/material";
import ArrowBackIcon from "@mui/icons-material/ArrowBack";
import { useNavigate, useParams } from "react-router-dom";
import { route } from "@/router/paths";
import { settingsPages } from "./pages";

export const SettingsPageRoute = route("page/:pageId");

/** The list of pages, each a row with an icon and a line of what is inside. */
export function MobileSettingsHome() {
  const navigate = useNavigate();
  return (
    <Box>
      <Typography variant="h5" sx={{ px: 2, pt: 3, pb: 1.5, fontWeight: 600 }}>Settings</Typography>
      {settingsPages.map(({ id, title, summary, icon }) => (
        <ListItemButton
          key={id}
          onClick={() => navigate(SettingsPageRoute.to({ pageId: id }))}
          sx={{ gap: 2.5, px: 2, py: 1.5 }}
        >
          <Box sx={{ display: "flex", color: "primary.main" }}>{icon}</Box>
          <Box sx={{ minWidth: 0 }}>
            <Typography>{title}</Typography>
            <Typography variant="body2" color="text.secondary">{summary}</Typography>
          </Box>
        </ListItemButton>
      ))}
    </Box>
  );
}

/** One page under a top bar with a back arrow. The bottom navigation is hidden, set on the route. */
export function MobileSettingsPage() {
  const navigate = useNavigate();
  const { pageId } = useParams();
  const page = settingsPages.find((p) => p.id === pageId);
  if (!page) return null;

  return (
    <Box>
      <Box
        sx={{
          position: "sticky", top: 0, zIndex: 1, display: "flex", alignItems: "center", gap: 1, px: 1, py: 1,
          bgcolor: "background.default",
        }}
      >
        <IconButton aria-label="Back" onClick={() => navigate(-1)}>
          <ArrowBackIcon />
        </IconButton>
        <Typography variant="h6">{page.title}</Typography>
      </Box>
      <page.Content />
    </Box>
  );
}
