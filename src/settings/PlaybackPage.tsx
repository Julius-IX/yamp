import { isBoolean, useSetting } from "@/lib/settings";
import type { AudioQuality } from "@/lib/types";
import { isQuality } from "@/player/store";
import { ChoiceRow, PlaceholderRow, SwitchRow } from "./controls";
import { Section } from "./rows";

const QUALITIES: { value: AudioQuality; label: string }[] = [
  { value: "low", label: "Low" },
  { value: "medium", label: "Medium" },
  { value: "high", label: "High" },
];

export default function PlaybackPage() {
  const [autoplay, setAutoplay] = useSetting("autoplay", true, isBoolean);
  const [quality, setQuality] = useSetting<AudioQuality>("quality", "high", isQuality);
  const [prefetch, setPrefetch] = useSetting("prefetch", true, isBoolean);

  return (
    <Section title="Playback">
      <SwitchRow
        title="Autoplay"
        summary="Continue playing similar songs when queue ends"
        checked={autoplay}
        onChange={setAutoplay}
      />
      <PlaceholderRow title="Discord Rich Presence" summary="Show what you're listening to on Discord" />
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
