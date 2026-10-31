import { useState } from "react";
import {
  Box, IconButton, List, ListItemButton, ListItemIcon, ListItemText, Paper, Tooltip, Typography,
} from "@mui/material";
import MenuOpenIcon from "@mui/icons-material/MenuOpen";
import type { Tab } from "@/router/types";
import AlbumList from "./AlbumList";
import SidebarSection from "./SidebarSection";

const WIDTH = 240;
/** Icon-only rail. */
const WIDTH_COMPACT = 64;

/**
 * Desktop navigation. The tabs stay at the top and the sections below them (the albums for
 * now) scroll on their own, since those lists can get far longer than the window.
 */
export default function Sidebar({
  tabs, activeId, onSelect,
}: { tabs: Tab[]; activeId: string; onSelect: (tab: Tab) => void }) {
  const [compact, setCompact] = useState(false);

  return (
    <Paper
      square
      elevation={0}
      sx={{
        display: "flex", flexDirection: "column", flexShrink: 0, overflow: "hidden",
        width: compact ? WIDTH_COMPACT : WIDTH, borderRight: 1, borderColor: "divider",
        transition: "width 0.25s cubic-bezier(0.4, 0, 0.2, 1)",
      }}
    >
      <Box sx={{ display: "flex", alignItems: "center", gap: 1, px: 1.5, pt: 1.5, pb: 1, justifyContent: compact ? "center" : "flex-start" }}>
        <IconButton size="small" aria-label="Toggle sidebar" onClick={() => setCompact((c) => !c)}>
          <MenuOpenIcon sx={{ transform: compact ? "scaleX(-1)" : "none" }} />
        </IconButton>
        {!compact && <Typography variant="subtitle1" noWrap sx={{ fontWeight: 700 }}>yamp</Typography>}
      </Box>

      <List sx={{ px: 1, flexShrink: 0 }}>
        {tabs.map((t) => (
          <Tooltip key={t.id} title={compact ? t.label : ""} placement="right">
            <ListItemButton
              selected={t.id === activeId}
              onClick={() => onSelect(t)}
              sx={{ borderRadius: 1, justifyContent: compact ? "center" : "flex-start" }}
            >
              <ListItemIcon sx={{ minWidth: compact ? 0 : 40 }}>{t.icon}</ListItemIcon>
              {!compact && <ListItemText primary={t.label} />}
            </ListItemButton>
          </Tooltip>
        ))}
      </List>

      <Box sx={{ flex: 1, minHeight: 0, overflowY: "auto", scrollbarWidth: "none", "&::-webkit-scrollbar": { display: "none" } }}>
        <SidebarSection title="Albums" compact={compact}>
          <AlbumList compact={compact} />
        </SidebarSection>
      </Box>
    </Paper>
  );
}
