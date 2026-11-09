import type { ReactNode } from "react";
import { Box, Typography } from "@mui/material";
import { GUTTER, sectionTitle, subdued } from "@/themes/ui";

/** A titled block of the radio screen. `empty` is shown under the title when the list came back with nothing. */
export default function Section({ title, empty, children }: { title: string; empty?: string; children: ReactNode }) {
  return (
    <Box component="section" sx={{ pb: 3, minWidth: 0 }}>
      <Box sx={{ px: GUTTER, pb: 1.5 }}>
        <Typography sx={sectionTitle}>{title}</Typography>
        {empty && <Typography variant="body2" sx={subdued}>{empty}</Typography>}
      </Box>
      {children}
    </Box>
  );
}
