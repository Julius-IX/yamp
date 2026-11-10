import { COMMANDS } from "./commands";
import { logger } from "./log";
import { getSetting, isRecord, onSettingChange, rawSetting, setSetting, useSetting } from "./settings";

/**
 * Settings that follow the user to their other devices. A section of the settings screen has a
 * switch, and while it is on, the settings listed here for the section are kept in the shared
 * key value store of the library link, see `values` in the backend. They travel with the next
 * sync like the rest of the library, the newest change wins.
 *
 * The switches themselves are saved on this device only. A device that is not linked, or has a
 * section off, neither sends nor takes over anything for it.
 */
const log = logger("cloud-settings");

const PREFIX = "setting:";

export type CloudSection = "general" | "appearance" | "playback" | "data";

type SectionInfo = {
  keys: readonly string[];
  /** Explains what is left out, shown with the switch. */
  note?: string;
};

export const CLOUD_SECTIONS: Record<CloudSection, SectionInfo> = {
  general: { keys: ["country"] },
  appearance: { keys: ["theme", "mode", "animations", "tooltips", "customThemes"] },
  playback: { keys: ["autoplay", "quality", "prefetch", "discordPresence"] },
  // folders and downloads belong to one device, so only the statistics options are shared
  data: { keys: ["statsEnabled"], note: "Only the statistics option is saved, folders and downloads stay on this device" },
};

const SWITCHES = "cloudSections";

// a stable fallback, a new object on every read would make the hook render forever
const NONE: Record<string, unknown> = {};
// a section that was never switched counts as on
const isOn = (section: CloudSection) => getSetting(SWITCHES, NONE, isRecord)[section] !== false;

const sectionOf = (key: string) => (Object.keys(CLOUD_SECTIONS) as CloudSection[]).find((s) => CLOUD_SECTIONS[s].keys.includes(key));

const send = (key: string, value: unknown) =>
  COMMANDS.values
    .set({ key: PREFIX + key, value: JSON.stringify(value) })
    .catch((e) => log.error(`couldn't save "${key}" to the cloud:`, e));

// a change made here goes out when its section is on
onSettingChange((key, value) => {
  const section = sectionOf(key);
  if (section && isOn(section)) void send(key, value);
});

const decode = (text: string): unknown => {
  try {
    return JSON.parse(text);
  } catch {
    return undefined;
  }
};

/** What the account holds, by setting name. */
async function stored() {
  const found = await COMMANDS.values.list({ prefix: PREFIX });
  return new Map(found.map((v) => [v.key.slice(PREFIX.length), decode(v.value)] as const));
}

/**
 * Takes over the values of the account for every section that is on. Run after a sync that
 * brought changes. A value that is already the same is left alone.
 */
export async function pullSettings() {
  try {
    const theirs = await stored();
    for (const [key, value] of theirs) {
      const section = sectionOf(key);
      if (!section || !isOn(section) || value === undefined) continue;
      if (JSON.stringify(rawSetting(key)) !== JSON.stringify(value)) setSetting(key, value, true);
    }
  } catch (e) {
    log.error("couldn't read the settings of the cloud:", e);
  }
}

/**
 * Turning a section on joins it to what the account already holds. A setting the account has takes
 * the place of the local one, which is what makes a second device pick up the first one's look.
 * A setting only this device has set is sent.
 */
async function join(section: CloudSection) {
  try {
    const theirs = await stored();
    for (const key of CLOUD_SECTIONS[section].keys) {
      const value = theirs.get(key);
      if (value !== undefined) {
        if (JSON.stringify(rawSetting(key)) !== JSON.stringify(value)) setSetting(key, value, true);
      } else if (rawSetting(key) !== undefined) {
        await send(key, rawSetting(key));
      }
    }
  } catch (e) {
    log.error(`couldn't join "${section}" to the cloud:`, e);
  }
}

/** The switch of a section. Turning it on joins what the account holds. */
export function useCloudSection(section: CloudSection) {
  const [switches, setSwitches] = useSetting(SWITCHES, NONE, isRecord);
  const set = (next: boolean) => {
    setSwitches({ ...switches, [section]: next });
    if (next) void join(section);
  };
  return [switches[section] !== false, set] as const;
}
