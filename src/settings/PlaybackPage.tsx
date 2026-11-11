import { useDiscordAvailable } from "@/lib/discord";
import { isBoolean, useSetting } from "@/lib/settings";
import type { AudioQuality } from "@/lib/types";
import { defaultPrefetchAmount, isPrefetchAmount, isQuality, type PrefetchAmount } from "@/player/store";
import { IS_MOBILE } from "@/util/platform";
import { ChoiceRow, SwitchRow } from "./controls";
import { PrefRow, Section } from "./rows";

const QUALITIES: { value: AudioQuality; label: string }[] = [
  { value: "low", label: "Low" },
  { value: "medium", label: "Medium" },
  { value: "high", label: "High" },
];

const PREFETCH: { value: PrefetchAmount; label: string }[] = [
  { value: "off", label: "Off" },
  { value: "1", label: "1 song" },
  { value: "3", label: "3 songs" },
  { value: "5", label: "5 songs" },
  { value: "10", label: "10 songs" },
  { value: "all", label: "Entire queue" },
];

export default function PlaybackPage() {
  const [autoplay, setAutoplay] = useSetting("autoplay", true, isBoolean);
  const [quality, setQuality] = useSetting<AudioQuality>("quality", "high", isQuality);
  const [prefetch, setPrefetch] = useSetting<PrefetchAmount>("prefetchAmount", defaultPrefetchAmount(), isPrefetchAmount);
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
      <ChoiceRow
        title="Prefetch"
        summary="How many upcoming songs are prepared ahead of time, for playback without a wait"
        value={prefetch}
        options={PREFETCH}
        onChange={setPrefetch}
      />
    </Section>
  );
}
