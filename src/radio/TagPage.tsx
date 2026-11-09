import { Box, IconButton, Typography } from "@mui/material";
import ArrowBackIcon from "@mui/icons-material/ArrowBack";
import { useNavigate, useParams } from "react-router-dom";
import { useCached } from "@/browse/useCached";
import { eyebrow, floating, GUTTER, heroTitle, subdued } from "@/themes/ui";
import { IS_MOBILE } from "@/util/platform";
import { searchStations } from "./api";
import StationGrid from "./StationGrid";

const LIMIT = 100;

/** The most played stations of one tag. */
export default function TagPage() {
  const { name = "" } = useParams<{ name: string }>();
  const navigate = useNavigate();
  const stations = useCached(`radio:tag:${name}`, () => searchStations({ tag: name, limit: LIMIT }), []);

  return (
    <Box sx={{ pt: IS_MOBILE ? 2 : 3, pb: 2 }}>
      <Box sx={{ px: GUTTER, pb: 3 }}>
        <IconButton aria-label="Back" onClick={() => navigate(-1)} sx={{ width: 36, height: 36, mb: 2, ...floating }}>
          <ArrowBackIcon fontSize="small" />
        </IconButton>
        <Typography sx={eyebrow}>TAG</Typography>
        <Typography sx={heroTitle}>{name}</Typography>
        {stations?.length === 0 && <Typography sx={{ ...subdued, pt: 1 }}>No stations could be loaded for this tag.</Typography>}
      </Box>
      <StationGrid stations={stations} numbered />
    </Box>
  );
}
