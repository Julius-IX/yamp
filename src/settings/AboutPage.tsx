import { useEffect, useState } from "react";
import GitHubIcon from "@mui/icons-material/GitHub";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { logger } from "@/lib/log";
import { ActionRow } from "./controls";
import { PrefRow, Section } from "./rows";

const log = logger("settings");

const REPO = "https://github.com/Julius-IX/yamp";

export default function AboutPage() {
  const [version, setVersion] = useState("");
  useEffect(() => {
    getVersion().then(setVersion).catch(() => setVersion("unknown"));
  }, []);

  return (
    <Section title="About">
      <PrefRow title="Version" summary={version || "..."} />
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
