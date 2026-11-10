import { Box, Divider, Typography } from "@mui/material";
import { settingsPages } from "./pages";

/**
 * Every section on one scrolling screen, in the order of the desktop layout. The sections hold
 * only a few options each, so a screen per section would be mostly empty. Each section carries
 * its own heading and a divider sets it apart from the one before. The dividers stop short of the
 * screen edges, like the line inside a card on desktop.
 */
export default function MobileSettings() {
  return (
    <Box>
      <Typography variant="h5" sx={{ px: 2, pt: 3, pb: 1.5, fontWeight: 600 }}>Settings</Typography>
      {settingsPages.map(({ id, Content }) => (
        <Box key={id} id={`settings-${id}`}>
          <Divider sx={{ mx: 2 }} />
          <Content />
        </Box>
      ))}
    </Box>
  );
}
