import { Box, Typography } from "@mui/material";
import { GUTTER } from "@/themes/ui";
import { IS_MOBILE } from "@/util/platform";
import FollowedReleases from "./FollowedReleases";
import QuickPicks from "./QuickPicks";
import Recommended from "./Recommended";
import RecentlyPlayed from "./RecentlyPlayed";

/** The small hours still belong to the night, so "morning" only starts once most people are up. */
const greeting = () => {
  const hour = new Date().getHours();
  if (hour < 5) return "Up late?";
  if (hour < 12) return "Good morning";
  if (hour < 18) return "Good afternoon";
  if (hour < 22) return "Good evening";
  return "Good night";
};

export default function HomePage() {
  return (
    // the search bar floats over the top, so on a narrow screen the greeting sits below it
    <Box sx={{ pt: IS_MOBILE ? 10 : 3, pb: 2, marginBottom: 1 }}>
      <Typography variant={IS_MOBILE ? "h5" : "h4"} sx={{ px: GUTTER, pb: 3, fontWeight: 800, lineHeight: "40px" }}>
        {greeting()}
      </Typography>
      <QuickPicks />
      <RecentlyPlayed />
      <FollowedReleases />
      <Recommended />
    </Box>
  );
}
