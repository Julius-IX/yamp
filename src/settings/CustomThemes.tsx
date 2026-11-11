import { useEffect, useState } from "react";
import {
  Box, Button, Dialog, DialogActions, DialogContent, DialogContentText, DialogTitle, IconButton, MenuItem, Select, Stack,
  TextField, ToggleButton, ToggleButtonGroup, Typography,
} from "@mui/material";
import { getContrastRatio } from "@mui/material/styles";
import DeleteOutlinedIcon from "@mui/icons-material/DeleteOutlined";
import EditOutlinedIcon from "@mui/icons-material/EditOutlined";
import { useBackGuard } from "@/router/useBackGuard";
import { useAppTheme } from "@/themes/AppThemeProvider";
import { COLOR_FIELDS, customThemeId, isHex, newThemeId, useCustomThemes, type CustomTheme } from "@/themes/custom";
import { builtinColors, DEFAULT_THEME, themeLabel, themeNames, type Colors, type Mode } from "@/themes/themes";
import { subdued } from "@/themes/ui";
import { IS_MOBILE } from "@/util/platform";
import { ActionRow } from "./controls";

const startFrom = (name: (typeof themeNames)[number]) => ({ dark: builtinColors(name, "dark"), light: builtinColors(name, "light") });

/** Like the real thing in small, so a color can be judged without leaving the editor. */
function Preview({ colors }: { colors: Colors }) {
  // the same rule MUI uses to pick the text on a filled accent
  const onAccent = getContrastRatio(colors.accent, "#fff") >= 3 ? "#fff" : "#000";
  const warnings = [
    getContrastRatio(colors.text, colors.surface) < 4.5 && "The text is hard to read on the surface",
    getContrastRatio(colors.accent, colors.surface) < 3 && "The accent is hard to see on the surface",
  ].filter(Boolean);
  return (
    <Box>
      <Box sx={{ bgcolor: colors.base, p: 1.5, borderRadius: 2, border: 1, borderColor: "divider" }}>
        <Box sx={{ bgcolor: colors.surface, borderRadius: 1.5, p: 1.5, display: "flex", alignItems: "center", gap: 1.5 }}>
          <Box sx={{ width: 44, height: 44, borderRadius: 1, bgcolor: colors.accent, flexShrink: 0 }} />
          <Box sx={{ flex: 1, minWidth: 0 }}>
            <Typography noWrap sx={{ color: colors.text, fontWeight: 600 }}>Song title</Typography>
            <Typography noWrap variant="body2" sx={{ color: colors.textSecondary }}>Artist</Typography>
            <Typography noWrap variant="caption" sx={{ color: colors.textSubdued }}>3:42</Typography>
          </Box>
          <Box sx={{ px: 1.75, py: 0.5, borderRadius: 99, bgcolor: colors.accent, color: onAccent, fontWeight: 600, fontSize: 14 }}>Play</Box>
        </Box>
      </Box>
      {warnings.map((w) => (
        <Typography key={String(w)} variant="caption" color="warning.main" sx={{ display: "block", mt: 0.5 }}>{w}</Typography>
      ))}
    </Box>
  );
}

/** A picker and a hex field for one color. The field only commits a complete, valid value. */
function ColorField({ label, value, onChange }: { label: string; value: string; onChange: (v: string) => void }) {
  const [text, setText] = useState(value);
  // the picker and "start from" change the value from outside
  useEffect(() => setText(value), [value]);
  return (
    <Box sx={{ display: "flex", alignItems: "center", gap: 1.5 }}>
      <Box
        component="input"
        type="color"
        aria-label={label}
        value={value.toLowerCase()}
        onChange={(e: React.ChangeEvent<HTMLInputElement>) => onChange(e.target.value)}
        sx={{ width: 44, height: 40, p: 0, border: 0, bgcolor: "transparent", cursor: "pointer", flexShrink: 0 }}
      />
      <TextField
        size="small"
        fullWidth
        label={label}
        value={text}
        error={!isHex(text)}
        onChange={(e) => {
          setText(e.target.value);
          if (isHex(e.target.value)) onChange(e.target.value);
        }}
      />
    </Box>
  );
}

function Editor({ theme, onSave, onClose }: { theme: CustomTheme | null; onSave: (t: CustomTheme) => void; onClose: () => void }) {
  useBackGuard(true, onClose);
  const [name, setName] = useState(theme?.name ?? "");
  const [colors, setColors] = useState(theme ?? { ...startFrom(DEFAULT_THEME) });
  const [mode, setMode] = useState<Mode>("dark");

  const set = (key: keyof Colors, value: string) => setColors((c) => ({ ...c, [mode]: { ...c[mode], [key]: value } }));

  return (
    <Dialog open onClose={onClose} fullWidth maxWidth="sm" fullScreen={IS_MOBILE}>
      {/* full screen on mobile, so the title starts at the top edge of the screen, behind the status bar */}
      <DialogTitle sx={IS_MOBILE ? { pt: "calc(env(safe-area-inset-top) + 16px)" } : undefined}>
        {theme ? "Edit theme" : "New theme"}
      </DialogTitle>
      <DialogContent>
        <Stack spacing={2} sx={{ pt: 1 }}>
          <TextField size="small" label="Name" value={name} onChange={(e) => setName(e.target.value)} slotProps={{ htmlInput: { maxLength: 40 } }} />
          {!theme && (
            <Select
              size="small"
              value=""
              displayEmpty
              renderValue={() => "Start from a built-in theme"}
              onChange={(e) => e.target.value && setColors(startFrom(e.target.value as (typeof themeNames)[number]))}
            >
              {themeNames.map((n) => (
                <MenuItem key={n} value={n}>{themeLabel(n)}</MenuItem>
              ))}
            </Select>
          )}
          <ToggleButtonGroup exclusive fullWidth size="small" value={mode} onChange={(_, v: Mode | null) => v && setMode(v)}>
            <ToggleButton value="dark" sx={{ textTransform: "none" }}>Dark mode</ToggleButton>
            <ToggleButton value="light" sx={{ textTransform: "none" }}>Light mode</ToggleButton>
          </ToggleButtonGroup>
          <Preview colors={colors[mode]} />
          {COLOR_FIELDS.map(({ key, label }) => (
            <ColorField key={`${mode}-${key}`} label={label} value={colors[mode][key]} onChange={(v) => set(key, v)} />
          ))}
        </Stack>
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>Cancel</Button>
        <Button
          variant="contained"
          onClick={() => onSave({ id: theme?.id ?? newThemeId(), name: name.trim() || "My theme", dark: colors.dark, light: colors.light })}
        >
          Save
        </Button>
      </DialogActions>
    </Dialog>
  );
}

/**
 * Lists the themes the user made and opens the editor for one. A theme is chosen in the theme
 * option above like a built-in one, a new theme is chosen when it is saved.
 */
export default function CustomThemesRow() {
  const { name: selected, setName } = useAppTheme();
  const [themes, setThemes] = useCustomThemes();
  const [managing, setManaging] = useState(false);
  const [editing, setEditing] = useState<CustomTheme | "new" | null>(null);
  const [deleting, setDeleting] = useState<CustomTheme | null>(null);
  useBackGuard(managing, () => setManaging(false));
  useBackGuard(deleting !== null, () => setDeleting(null));

  const save = (t: CustomTheme) => {
    const exists = themes.some((x) => x.id === t.id);
    setThemes(exists ? themes.map((x) => (x.id === t.id ? t : x)) : [...themes, t]);
    if (!exists) setName(customThemeId(t.id));
    setEditing(null);
  };

  const remove = () => {
    if (!deleting) return;
    setThemes(themes.filter((t) => t.id !== deleting.id));
    // a theme that is gone cannot stay selected
    if (selected === customThemeId(deleting.id)) setName(DEFAULT_THEME);
    setDeleting(null);
  };

  return (
    <>
      <ActionRow
        title="Custom themes"
        summary={themes.length ? `${themes.length} of your own` : "Add custom color combinations"}
        label="Manage"
        onAction={() => setManaging(true)}
      />
      <Dialog open={managing} onClose={() => setManaging(false)} fullWidth maxWidth="xs">
        <DialogTitle>Custom themes</DialogTitle>
        <DialogContent sx={{ px: 1 }}>
          {themes.length === 0 && <Typography sx={{ ...subdued, px: 2 }}>No custom themes yet.</Typography>}
          {themes.map((t) => (
            <Box key={t.id} sx={{ display: "flex", alignItems: "center", gap: 1.5, px: 2, py: 0.5 }}>
              <Box sx={{ width: 28, height: 28, borderRadius: "50%", flexShrink: 0, bgcolor: t.dark.accent, border: 3, borderColor: t.dark.base }} />
              <Typography noWrap sx={{ flex: 1, minWidth: 0 }}>{t.name}</Typography>
              <IconButton aria-label={`Edit ${t.name}`} onClick={() => setEditing(t)}><EditOutlinedIcon /></IconButton>
              <IconButton aria-label={`Delete ${t.name}`} onClick={() => setDeleting(t)}><DeleteOutlinedIcon /></IconButton>
            </Box>
          ))}
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setManaging(false)}>Close</Button>
          <Button variant="contained" onClick={() => setEditing("new")}>New theme</Button>
        </DialogActions>
      </Dialog>
      {editing && <Editor theme={editing === "new" ? null : editing} onSave={save} onClose={() => setEditing(null)} />}
      <Dialog open={deleting !== null} onClose={() => setDeleting(null)} fullWidth maxWidth="xs">
        <DialogTitle>Delete theme</DialogTitle>
        <DialogContent>
          <DialogContentText>{deleting ? `"${deleting.name}" is deleted from every device that shares the appearance settings.` : ""}</DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setDeleting(null)}>Cancel</Button>
          <Button color="error" variant="contained" onClick={remove}>Delete</Button>
        </DialogActions>
      </Dialog>
    </>
  );
}
