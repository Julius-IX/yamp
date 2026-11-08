import { ReactNode, useState } from "react";
import { Box, ButtonBase, Collapse, Typography } from "@mui/material";
import ExpandMoreIcon from "@mui/icons-material/ExpandMore";

/**
 * A titled part of the sidebar that can be folded. In the icon-only rail there is no room for
 * the title or the action, so the items are shown and cannot be folded.
 */
export default function SidebarSection({
  title, compact, action, children,
}: { title: string; compact: boolean; /** A button at the end of the title row. */ action?: ReactNode; children: ReactNode }) {
  const [open, setOpen] = useState(true);

  if (compact) return <Box sx={{ px: 0.5 }}>{children}</Box>;

  return (
    <Box sx={{ px: 1 }}>
      <Box sx={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
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
        {action}
      </Box>
      <Collapse in={open}>{children}</Collapse>
    </Box>
  );
}
