import { useCallback, useEffect, useState } from "react";
import { Box, Button, Chip, CircularProgress } from "@mui/material";
import RefreshIcon from "@mui/icons-material/Refresh";
import SystemUpdateAltIcon from "@mui/icons-material/SystemUpdateAlt";
import { COMMANDS } from "../../lib/commands";
import type { EngineStatus } from "../../lib/types";

/** State of the provider tooling (yt-dlp or its Android equivalent) with a manual update. */
export default function EngineBar({ onError }: { onError: (message: string) => void }) {
  const [status, setStatus] = useState<EngineStatus | null>(null);
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(
    () => COMMANDS.engine.status().then(setStatus).catch((e) => onError(`engine status: ${e}`)),
    [onError],
  );

  useEffect(() => {
    refresh();
  }, [refresh]);

  const update = async () => {
    setBusy(true);
    try {
      setStatus(await COMMANDS.engine.update());
    } catch (e) {
      onError(`engine update: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
      <Chip
        size="small"
        color={status?.installed ? "success" : "warning"}
        label={
          status === null
            ? "checking..."
            : status.installed
              ? `yt-dlp ${status.version}`
              : "not installed yet, it installs on first launch"
        }
      />
      <Button size="small" startIcon={<RefreshIcon />} onClick={refresh} disabled={busy}>
        Refresh
      </Button>
      <Button
        size="small"
        startIcon={busy ? <CircularProgress size={14} /> : <SystemUpdateAltIcon />}
        onClick={update}
        disabled={busy}
      >
        {status?.installed ? "Update" : "Install"}
      </Button>
    </Box>
  );
}
