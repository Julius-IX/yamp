import { Box, Typography } from "@mui/material";
import { GUTTER } from "@/themes/ui";
import { IS_MOBILE } from "@/util/platform";
import CountryHint from "./CountryHint";
import Moods from "./Moods";
import NewReleases from "./NewReleases";
import NewVideos from "./NewVideos";
import TopArtists from "./TopArtists";
import Trending from "./Trending";

export default function ExplorePage() {
  return (
    // the search bar floats over the top, so on a narrow screen the title sits below it
    <Box sx={{ pt: IS_MOBILE ? 10 : 3, pb: 2, marginBottom: 1 }}>
      <Typography variant={IS_MOBILE ? "h5" : "h4"} sx={{ px: GUTTER, pb: 3, fontWeight: 800, lineHeight: "40px" }}>
        Explore
      </Typography>
      <CountryHint />
      <NewReleases />
      <Trending />
      <TopArtists />
      <NewVideos />
      <Moods />
    </Box>
  );
}
