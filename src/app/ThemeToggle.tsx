import { MenuItem, Select, Stack, ToggleButton, ToggleButtonGroup } from "@mui/material";
import { useAppTheme } from "../themes/AppThemeProvider";
import { modeLabel, modePrefs, themeLabel, themeNames, type ThemeName } from "../themes/themes";

// Placeholder until there is a settings page.
export default function ThemeToggle() {
  const { name, setName, mode, setMode } = useAppTheme();
  return (
    <Stack direction="row" spacing={2} sx={{ p: 2 }}>
      <Select size="small" value={name} onChange={(e) => setName(e.target.value as ThemeName)}>
        {themeNames.map((n) => (
          <MenuItem key={n} value={n}>
            {themeLabel(n)}
          </MenuItem>
        ))}
      </Select>
      <ToggleButtonGroup
        size="small"
        exclusive
        value={mode}
        onChange={(_, m) => m && setMode(m)}
      >
        {modePrefs.map((m) => (
          <ToggleButton key={m} value={m}>
            {modeLabel(m)}
          </ToggleButton>
        ))}
      </ToggleButtonGroup>
    </Stack>
  );
}
