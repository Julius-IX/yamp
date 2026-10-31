import { ReactNode, useState } from "react";
import { Box, ButtonBase, Collapse, Typography } from "@mui/material";
import ExpandMoreIcon from "@mui/icons-material/ExpandMore";

/**
 * A titled part of the sidebar that can be folded. In the icon-only rail there is no room for
 * the title, so the items are shown and cannot be folded.
 */
export default function SidebarSection({
  title, compact, children,
}: { title: string; compact: boolean; children: ReactNode }) {
  const [open, setOpen] = useState(true);

  if (compact) return <Box sx={{ px: 0.5 }}>{children}</Box>;

  return (
    <Box sx={{ px: 1 }}>
      <ButtonBase
        onClick={() => setOpen((o) => !o)}
        sx={{ display: "flex", alignItems: "center", gap: 0.5, px: 1, py: 0.5, borderRadius: 1, color: "text.secondary" }}
      >
        <ExpandMoreIcon
          fontSize="small"
          sx={{ transition: "transform 0.2s", transform: open ? "none" : "rotate(-90deg)" }}
        />
        <Typography variant="overline" sx={{ lineHeight: 1.8 }}>{title}</Typography>
      </ButtonBase>
      <Collapse in={open}>{children}</Collapse>
    </Box>
  );
}
