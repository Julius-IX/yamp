import { useMemo } from "react";
import { type Theme } from "@mui/material/styles";
import { useSetting } from "@/lib/settings";
import { buildTheme, DEFAULT_THEME, themes, type Colors, type Mode, type ThemeName } from "./themes";

/**
 * Themes made by the user. They are the same thing as the built-in ones, a set of colors for dark
 * and one for light, and go through the same `buildTheme`. They are saved as a list in the
 * settings, so they are also what the appearance section shares with the other devices.
 *
 * The selected theme is a built-in name, or `custom:` and the id of a custom one.
 */
export type CustomTheme = { id: string; name: string; dark: Colors; light: Colors };

const PREFIX = "custom:";

export const customThemeId = (id: string) => PREFIX + id;

/** The colors in the order the editor lists them. */
export const COLOR_FIELDS: { key: keyof Colors; label: string }[] = [
  { key: "accent", label: "Accent" },
  { key: "base", label: "Background" },
  { key: "surface", label: "Surface" },
  { key: "text", label: "Text" },
  { key: "textSecondary", label: "Secondary text" },
  { key: "textSubdued", label: "Subdued text" },
];

export const isHex = (v: unknown): v is string => typeof v === "string" && /^#[0-9a-f]{6}$/i.test(v);

const isColors = (v: unknown): v is Colors =>
  typeof v === "object" && v !== null && COLOR_FIELDS.every(({ key }) => isHex((v as Record<string, unknown>)[key]));

/** A list that came from a hand edited file or another device is dropped whole when one entry is off. */
export const isCustomThemes = (v: unknown): v is CustomTheme[] =>
  Array.isArray(v) &&
  v.every((t) => typeof t?.id === "string" && typeof t?.name === "string" && isColors(t?.dark) && isColors(t?.light));

// A stable fallback, a new array on every read would make the hook render forever.
const NONE: CustomTheme[] = [];

export const KEY = "customThemes";

export const useCustomThemes = () => useSetting<CustomTheme[]>(KEY, NONE, isCustomThemes);

export const newThemeId = () => Math.random().toString(36).slice(2, 10);

const built = new Map<string, Theme>();

/**
 * The theme for a selection. A selection that no longer exists, like a custom theme that was
 * deleted on another device, falls back to the default.
 */
export function resolveTheme(name: string, mode: Mode, customs: CustomTheme[]): Theme {
  if (name in themes) return themes[name as ThemeName][mode];
  const custom = name.startsWith(PREFIX) ? customs.find((t) => customThemeId(t.id) === name) : undefined;
  if (!custom) return themes[DEFAULT_THEME][mode];
  // built once per set of colors, since a theme is expensive to make and is read on every render
  const key = `${mode}${JSON.stringify(custom[mode])}`;
  let theme = built.get(key);
  if (!theme) built.set(key, (theme = buildTheme(mode, custom[mode])));
  return theme;
}

export function useResolvedTheme(name: string, mode: Mode) {
  const [customs] = useCustomThemes();
  return useMemo(() => resolveTheme(name, mode, customs), [name, mode, customs]);
}
