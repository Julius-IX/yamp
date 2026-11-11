// Keeps yt-dlp current. Providers change their sites often, and an old yt-dlp is what makes
// searches and streams start failing, so it is updated without anyone having to ask.
import { useEffect } from "react";
import { COMMANDS } from "./commands";
import { logger } from "./log";
import { getSetting, isBoolean, isNumber, setSetting, useSetting } from "./settings";

const log = logger("engine");

const FIRST_LOOK_MS = 60_000;
/** How often the time since the last update is looked at. A look is cheap, an update is not. */
const LOOK_EVERY_MS = 60 * 60 * 1000;
const UPDATE_EVERY_MS = 24 * 60 * 60 * 1000;

/** Remembered across launches, so restarting the app does not download yt-dlp every time. */
export const markEngineUpdated = () => setSetting("engineUpdatedMs", Date.now());

let running = false;

async function updateIfDue() {
  const due = Date.now() - getSetting("engineUpdatedMs", 0, isNumber) >= UPDATE_EVERY_MS;
  if (running || !due || !navigator.onLine) return;
  running = true;
  try {
    await COMMANDS.engine.update();
    markEngineUpdated();
  } catch (e) {
    // the backend logs the failure too, and the next look tries again
    log.warn("couldn't update yt-dlp:", e);
  } finally {
    running = false;
  }
}

/** Mount once. Updates yt-dlp about once a day while the option is on. */
export function useAutoEngineUpdate() {
  const [auto] = useSetting("engineAutoUpdate", true, isBoolean);
  useEffect(() => {
    if (!auto) return;
    const first = setTimeout(() => void updateIfDue(), FIRST_LOOK_MS);
    const timer = setInterval(() => void updateIfDue(), LOOK_EVERY_MS);
    return () => {
      clearTimeout(first);
      clearInterval(timer);
    };
  }, [auto]);
}
