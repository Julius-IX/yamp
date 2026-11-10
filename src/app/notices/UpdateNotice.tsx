import { Box, Button, LinearProgress } from "@mui/material";
import {
  installUpdate, restartToUpdate, skipVersion, useSkippedVersion, useUpdateStatus,
} from "@/lib/updater";
import { IS_ANDROID } from "@/util/platform";
import Notice from "./Notice";

/**
 * Offers a new version without forcing it. "Not now" keeps the notice away until a newer version
 * comes out. The About page still offers the update, since it is not tied to this choice.
 */
export default function UpdateNotice() {
  const status = useUpdateStatus();
  const skipped = useSkippedVersion();

  if (status.kind === "found" && status.update.version !== skipped) {
    const { version } = status.update;
    return (
      <Notice title="Update available" text={`yamp ${version} is ready to ${IS_ANDROID ? "download" : "install"}.`}>
        <Box sx={{ display: "flex", justifyContent: "flex-end", gap: 1 }}>
          <Button size="small" color="inherit" onClick={() => skipVersion(version)}>Not now</Button>
          <Button size="small" variant="contained" onClick={() => void installUpdate()}>
            {IS_ANDROID ? "Download" : "Update"}
          </Button>
        </Box>
      </Notice>
    );
  }
  if (status.kind === "installing") {
    const { downloaded, total } = status;
    return (
      <Notice title={`Updating to ${status.update.version}`} text="The app keeps working while it downloads.">
        <LinearProgress
          variant={total ? "determinate" : "indeterminate"}
          value={total ? Math.min(100, (downloaded / total) * 100) : 0}
        />
      </Notice>
    );
  }
  if (status.kind === "ready") {
    return (
      <Notice title="Update installed" text="Restart to start using the new version.">
        <Box sx={{ display: "flex", justifyContent: "flex-end" }}>
          <Button size="small" variant="contained" onClick={() => void restartToUpdate()}>Restart</Button>
        </Box>
      </Notice>
    );
  }
  return null;
}
