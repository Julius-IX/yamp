import { Box, Typography } from "@mui/material";
import { GUTTER } from "@/themes/ui";
import { IS_MOBILE } from "@/util/platform";
import FollowedReleases from "./FollowedReleases";
import QuickPicks from "./QuickPicks";
import Recommended from "./Recommended";
import RecentlyPlayed from "./RecentlyPlayed";

const greeting = () => {
  const hour = new Date().getHours();
  return hour < 12 ? "Good morning" : hour < 18 ? "Good afternoon" : "Good evening";
};

export default function HomePage() {
  return (
    // the search bar floats over the top, so on a narrow screen the greeting sits below it
    <Box sx={{ pt: IS_MOBILE ? 10 : 3, pb: 2 }}>
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
