import { useDiscordAvailable } from "@/lib/discord";
import { isBoolean, useSetting } from "@/lib/settings";
import type { AudioQuality } from "@/lib/types";
import { isQuality } from "@/player/store";
import { IS_MOBILE } from "@/util/platform";
import { ChoiceRow, SwitchRow } from "./controls";
import { PrefRow, Section } from "./rows";

const QUALITIES: { value: AudioQuality; label: string }[] = [
  { value: "low", label: "Low" },
  { value: "medium", label: "Medium" },
  { value: "high", label: "High" },
];

export default function PlaybackPage() {
  const [autoplay, setAutoplay] = useSetting("autoplay", true, isBoolean);
  const [quality, setQuality] = useSetting<AudioQuality>("quality", "high", isQuality);
  const [prefetch, setPrefetch] = useSetting("prefetch", true, isBoolean);
  const [discord, setDiscord] = useSetting("discordPresence", false, isBoolean);
  const discordAvailable = useDiscordAvailable();

  return (
    <Section title="Playback" cloud="playback">
      <SwitchRow
        title="Autoplay"
        summary="Continue playing similar songs when queue ends"
        checked={autoplay}
        onChange={setAutoplay}
      />
      {/* Discord runs on a desktop, there is nothing to show on a phone */}
      {!IS_MOBILE &&
        (discordAvailable ? (
          <SwitchRow
            title="Discord Rich Presence"
            summary="Show what you're listening to on Discord"
            checked={discord}
            onChange={setDiscord}
          />
        ) : (
          <PrefRow title="Discord Rich Presence" summary="No Discord application is set up in this build" />
        ))}
      <ChoiceRow
        title="Audio quality"
        summary="Higher quality uses more bandwidth"
        value={quality}
        options={QUALITIES}
        onChange={setQuality}
      />
      <SwitchRow
        title="Prefetch cache"
        summary="Pre-download upcoming tracks for uninterrupted playback"
        checked={prefetch}
        onChange={setPrefetch}
      />
    </Section>
  );
}
