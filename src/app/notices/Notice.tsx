import type { ReactNode } from "react";
import { Box, Paper, Typography } from "@mui/material";
import { subdued } from "@/themes/ui";

/** The frame of a notice, a title and a line under it, with room for buttons or a bar below. */
export default function Notice({ title, text, children }: { title: string; text?: string; children?: ReactNode }) {
  return (
    <Paper elevation={6} sx={{ p: 1.5, borderRadius: 2 }}>
      <Typography sx={{ fontWeight: 600 }}>{title}</Typography>
      {text && (
        <Typography variant="body2" sx={{ ...subdued, wordBreak: "break-word" }}>
          {text}
        </Typography>
      )}
      {children && <Box sx={{ mt: 1 }}>{children}</Box>}
    </Paper>
  );
}
