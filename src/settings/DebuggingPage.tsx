import { useEffect, useState } from "react";
import { Box } from "@mui/material";
import LogView from "@/components/LogView";
import { COMMANDS } from "@/lib/commands";
import { markEngineUpdated } from "@/lib/engine";
import { logger } from "@/lib/log";
import { isBoolean, useSetting } from "@/lib/settings";
import type { EngineStatus } from "@/lib/types";
import { IS_MOBILE } from "@/util/platform";
import { ActionRow, SwitchRow } from "./controls";
import { PrefRow, Section } from "./rows";

const log = logger("settings");

export default function DebuggingPage() {
  const [engine, setEngine] = useState<EngineStatus | null>(null);
  const [updating, setUpdating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [auto, setAuto] = useSetting("engineAutoUpdate", true, isBoolean);

  useEffect(() => {
    COMMANDS.engine.status().then(setEngine).catch((e) => setError(String(e)));
  }, []);

  const update = async () => {
    setUpdating(true);
    setError(null);
    try {
      setEngine(await COMMANDS.engine.update());
      markEngineUpdated();
    } catch (e) {
      log.error("couldn't update the engine:", e);
      setError(`Update failed: ${e}`);
    } finally {
      setUpdating(false);
    }
  };

  const version = engine === null ? "Checking..." : engine.installed ? (engine.version ?? "Unknown version") : "Not installed";
  return (
    <Section title="Debugging">
      <ActionRow
        title={engine?.installed === false ? "Install yt-dlp" : "Update yt-dlp"}
        summary={error ?? version}
        label={engine?.installed === false ? "Install" : "Update"}
        busy={updating}
        onAction={update}
      />
      <SwitchRow
        title="Update yt-dlp automatically"
        summary="Once a day, so searches and streams keep working when a site changes"
        checked={auto}
        onChange={setAuto}
      />
      <PrefRow title="Logs" />
      {/* the view fills its parent, so it needs a height of its own inside a scrolling page */}
      <Box sx={{ height: 420, px: IS_MOBILE ? 2 : 0 }}>
        <LogView />
      </Box>
    </Section>
  );
}
