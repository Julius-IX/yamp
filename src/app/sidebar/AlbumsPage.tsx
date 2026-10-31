import { Box, Typography } from "@mui/material";
import AlbumList from "./AlbumList";

/** Mobile only. Desktop lists the albums in the sidebar, here they get a tab of their own. */
export default function AlbumsPage() {
  return (
    <Box sx={{ p: 2 }}>
      <Typography variant="h5" sx={{ pb: 1 }}>Albums</Typography>
      <AlbumList compact={false} />
    </Box>
  );
}
