import { isBoolean, useSetting } from "@/lib/settings";
import { useAppTheme } from "@/themes/AppThemeProvider";
import { modeLabel, modePrefs, themeLabel, themeNames } from "@/themes/themes";
import { customThemeId, useCustomThemes } from "@/themes/custom";
import { ChoiceRow, SwitchRow } from "./controls";
import CustomThemesRow from "./CustomThemes";
import { Section } from "./rows";

const builtIn = themeNames.map((value) => ({ value: value as string, label: themeLabel(value) }));
const modes = modePrefs.map((value) => ({ value, label: modeLabel(value) }));

export default function AppearancePage() {
  const { name, setName, mode, setMode } = useAppTheme();
  const [custom] = useCustomThemes();
  const themes = [...builtIn, ...custom.map((t) => ({ value: customThemeId(t.id), label: t.name }))];
  const [animations, setAnimations] = useSetting("animations", true, isBoolean);
  const [tooltips, setTooltips] = useSetting("tooltips", true, isBoolean);
  return (
    <Section title="Appearance" cloud="appearance">
      <ChoiceRow
        title="Theme"
        summary="Choose a color theme for the app"
        value={name}
        options={themes}
        onChange={setName}
      />
      <ChoiceRow title="Mode" summary="Dark, light or the setting of the system" value={mode} options={modes} onChange={setMode} />
      <CustomThemesRow />
      <SwitchRow
        title="Animations"
        summary="Enable smooth transitions and animations throughout the app"
        checked={animations}
        onChange={setAnimations}
      />
      <SwitchRow
        title="Tooltips"
        summary="Show a short label on buttons that only have an icon"
        checked={tooltips}
        onChange={setTooltips}
      />
    </Section>
  );
}
