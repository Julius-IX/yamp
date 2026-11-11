import { useEffect } from "react";
import { Box, Button, IconButton, Paper, Typography } from "@mui/material";
import CloseIcon from "@mui/icons-material/Close";
import ErrorOutlineIcon from "@mui/icons-material/ErrorOutlined";
import { dismissError, useErrorNotices, watchErrors } from "@/lib/errors";
import { useTabs } from "@/router/TabsContext";

/** Where the log view is, which the notice offers to open. */
const LOGS = "settings-debugging";

/**
 * Tells that something went wrong, so nobody has to open the settings to find out. Only errors
 * come here. The notice goes away by itself, and the log view has the whole message.
 */
export default function ErrorNotice() {
  const notices = useErrorNotices();
  const { openTab } = useTabs();
  useEffect(watchErrors, []);

  const viewLogs = (id: number) => {
    dismissError(id);
    openTab("settings");
    // the settings tab is hidden until it is switched to, so it can only scroll once it is shown
    setTimeout(() => document.getElementById(LOGS)?.scrollIntoView({ block: "start" }), 50);
  };

  return (
    <>
      {notices.map((n) => (
        <Paper
          key={n.id}
          role="alert"
          elevation={6}
          sx={{ p: 1.5, borderRadius: 2, borderLeft: 4, borderColor: "error.main", display: "flex", gap: 1, alignItems: "flex-start" }}
        >
          <ErrorOutlineIcon color="error" sx={{ mt: 0.25 }} />
          <Box sx={{ flex: 1, minWidth: 0 }}>
            <Typography sx={{ fontWeight: 600 }}>
              Something went wrong{n.count > 1 ? ` (${n.count})` : ""}
            </Typography>
            <Typography variant="body2" color="text.secondary" sx={{ wordBreak: "break-word" }}>{n.message}</Typography>
            <Button size="small" color="inherit" onClick={() => viewLogs(n.id)} sx={{ mt: 0.5, ml: -1 }}>View logs</Button>
          </Box>
          <IconButton size="small" aria-label="Dismiss" onClick={() => dismissError(n.id)}>
            <CloseIcon fontSize="small" />
          </IconButton>
        </Paper>
      ))}
    </>
  );
}
