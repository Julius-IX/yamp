import { isBoolean, useSetting } from "@/lib/settings";
import { useAppTheme } from "@/themes/AppThemeProvider";
import { modeLabel, modePrefs, themeLabel, themeNames } from "@/themes/themes";
import { ChoiceRow, PlaceholderRow, SwitchRow } from "./controls";
import { Section } from "./rows";

const themes = themeNames.map((value) => ({ value, label: themeLabel(value) }));
const modes = modePrefs.map((value) => ({ value, label: modeLabel(value) }));

export default function AppearancePage() {
  const { name, setName, mode, setMode } = useAppTheme();
  const [animations, setAnimations] = useSetting("animations", true, isBoolean);
  return (
    <Section title="Appearance" mobileHeading={false}>
      <ChoiceRow
        title="Theme"
        summary="Choose a color theme for the app"
        value={name}
        options={themes}
        onChange={setName}
      />
      <ChoiceRow title="Mode" summary="Dark, light or the setting of the system" value={mode} options={modes} onChange={setMode} />
      <PlaceholderRow title="Custom Themes" summary="Add custom color combinations" />
      <SwitchRow
        title="Animations"
        summary="Enable smooth transitions and animations throughout the app"
        checked={animations}
        onChange={setAnimations}
      />
    </Section>
  );
}
