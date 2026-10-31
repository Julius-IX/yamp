import { Box, Typography } from "@mui/material";
import type { RouteObject } from "react-router-dom";

function Placeholder({ title }: { title: string }) {
  return (
    <Box sx={{ p: 2 }}>
      <Typography variant="h5">{title}</Typography>
    </Box>
  );
}

/** A tab with a single empty screen, until its feature is built. */
export const placeholderRoutes = (title: string): RouteObject[] => [
  { index: true, element: <Placeholder title={title} /> },
];
