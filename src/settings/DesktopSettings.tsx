import { Box, Typography } from "@mui/material";
import { settingsPages } from "./pages";

/** Every page on one scrolling screen. The sections carry their own titles, so there is no heading per page. */
export default function DesktopSettings() {
  return (
    <Box sx={{ maxWidth: 700, mx: "auto", px: 3, py: 4 }}>
      <Typography variant="h4" sx={{ fontWeight: 700, mb: 3 }}>Settings</Typography>
      {settingsPages.map(({ id, Content }) => (
        // the id lets a notice scroll to a section
        <Box key={id} id={`settings-${id}`}>
          <Content />
        </Box>
      ))}
    </Box>
  );
}
