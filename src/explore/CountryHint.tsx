import { Box, Link, Typography } from "@mui/material";
import PublicIcon from "@mui/icons-material/Public";
import { useCountry } from "@/lib/country";
import { useTabs } from "@/router/TabsContext";
import { faint, GUTTER, panel, subdued } from "@/themes/ui";

/** Shown until a country is chosen, since the charts are worldwide without one. */
export default function CountryHint() {
  const [country] = useCountry();
  const { openTab } = useTabs();
  if (country) return null;

  return (
    <Box sx={{ display: "flex", alignItems: "center", gap: 1.5, mx: GUTTER, mb: 3, px: 2, py: 1.25, ...panel }}>
      <PublicIcon fontSize="small" sx={faint} />
      <Typography variant="body2" sx={subdued}>
        Set your country in{" "}
        <Link component="button" type="button" underline="hover" onClick={() => openTab("settings")} sx={{ verticalAlign: "baseline" }}>
          Settings
        </Link>{" "}
        for more relevant recommendations
      </Typography>
    </Box>
  );
}
