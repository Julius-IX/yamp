import { useState } from "react";
import { Button, Dialog, DialogActions, DialogContent, DialogContentText, DialogTitle } from "@mui/material";
import { useNavigate } from "react-router-dom";
import { logger } from "@/lib/log";
import { isBoolean, useSetting } from "@/lib/settings";
import { useBackGuard } from "@/router/useBackGuard";
import { clearStats } from "@/stats/record";
import { ActionRow, SwitchRow } from "./controls";
import { Statistics } from "./paths";

const log = logger("settings");

/**
 * The statistics options of the data section. Viewing them is a row here and not a tab of its own,
 * it opens the statistics screen on top of the settings.
 */
export default function StatsRows() {
  const navigate = useNavigate();
  const [collect, setCollect] = useSetting("statsEnabled", true, isBoolean);
  const [clearing, setClearing] = useState(false);
  useBackGuard(clearing, () => setClearing(false));

  const clear = async () => {
    setClearing(false);
    try {
      await clearStats();
    } catch (e) {
      log.error("couldn't clear the statistics:", e);
    }
  };

  return (
    <>
      <SwitchRow
        title="Collect statistics"
        summary="Keep track of what is played to build listening statistics"
        checked={collect}
        onChange={setCollect}
      />
      <ActionRow
        title="Statistics"
        summary="Listening time, top songs and artists"
        label="View"
        onAction={() => navigate(Statistics.to())}
      />
      <ActionRow
        title="Clear statistics"
        summary="Removes the collected statistics of every linked device"
        label="Clear"
        danger
        onAction={() => setClearing(true)}
      />
      <Dialog open={clearing} onClose={() => setClearing(false)} fullWidth maxWidth="xs">
        <DialogTitle>Clear statistics</DialogTitle>
        <DialogContent>
          <DialogContentText>All listening statistics are deleted, on the devices that share this library too. This cannot be undone.</DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setClearing(false)}>Cancel</Button>
          <Button color="error" variant="contained" onClick={() => void clear()}>Clear</Button>
        </DialogActions>
      </Dialog>
    </>
  );
}
