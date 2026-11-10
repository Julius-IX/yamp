import { useEffect, useState } from "react";
import GitHubIcon from "@mui/icons-material/GitHub";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { logger } from "@/lib/log";
import { isBoolean, useSetting } from "@/lib/settings";
import {
  checkForUpdates, installUpdate, restartToUpdate, useUpdateStatus, useUpdatesAvailable, type UpdateStatus,
} from "@/lib/updater";
import { IS_ANDROID } from "@/util/platform";
import { ActionRow, SwitchRow } from "./controls";
import { PrefRow, Section } from "./rows";

const log = logger("settings");

const REPO = "https://github.com/Julius-IX/yamp";

/** What the update row says and does for each state of the updater. */
function updateRow(status: UpdateStatus) {
  switch (status.kind) {
    case "checking":
      return { summary: "Checking...", label: "Check", busy: true, onAction: () => {} };
    case "found":
      return {
        summary: `Version ${status.update.version} is available`,
        label: IS_ANDROID ? "Download" : "Update",
        busy: false,
        onAction: () => void installUpdate(),
      };
    case "installing": {
      const percent = status.total ? ` ${Math.round((status.downloaded / status.total) * 100)}%` : "";
      return { summary: `Downloading version ${status.update.version}${percent}`, label: "Update", busy: true, onAction: () => {} };
    }
    case "ready":
      return { summary: "Restart to start using the new version", label: "Restart", busy: false, onAction: () => void restartToUpdate() };
    case "current":
      return { summary: "yamp is up to date", label: "Check", busy: false, onAction: () => void checkForUpdates(true) };
    case "failed":
      return { summary: "Couldn't check, see the logs", label: "Retry", busy: false, onAction: () => void checkForUpdates(true) };
    default:
      return { summary: "Look for a new version", label: "Check", busy: false, onAction: () => void checkForUpdates(true) };
  }
}

export default function AboutPage() {
  const [version, setVersion] = useState("");
  const [auto, setAuto] = useSetting("updateCheck", true, isBoolean);
  const available = useUpdatesAvailable();
  const row = updateRow(useUpdateStatus());
  useEffect(() => {
    getVersion().then(setVersion).catch(() => setVersion("unknown"));
  }, []);

  return (
    <Section title="About">
      <PrefRow title="Version" summary={version || "..."} />
      {available ? (
        <>
          <ActionRow title="Updates" summary={row.summary} label={row.label} busy={row.busy} onAction={row.onAction} />
          <SwitchRow
            title="Check for updates automatically"
            summary="Looks when yamp starts and every few hours, nothing is installed without asking"
            checked={auto}
            onChange={setAuto}
          />
        </>
      ) : (
        <PrefRow title="Updates" summary="Not available in this install" />
      )}
      <ActionRow
        title="Github"
        icon={<GitHubIcon />}
        summary="github.com/Julius-IX/yamp"
        label="Open"
        onAction={() => openUrl(REPO).catch((e) => log.error("couldn't open the repository:", e))}
      />
    </Section>
  );
}
