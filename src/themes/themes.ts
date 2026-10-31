import { createTheme, type PaletteOptions, type Theme } from "@mui/material/styles";

export type Mode = "light" | "dark";

/** What the user picks. `system` follows the OS and resolves to one of `Mode`. */
export const modePrefs = ["system", "dark", "light"] as const;
export type ModePref = (typeof modePrefs)[number];
export const DEFAULT_MODE: ModePref = "dark";

type Colors = {
  accent: string;
  base: string;
  surface: string;
  text: string;
  textSecondary: string;
  textSubdued: string;
};

// Snowify keeps one red for every theme, including the light one.
const ERROR = "#f06070";

const palette = (c: Colors, divider: string): PaletteOptions => ({
  primary: { main: c.accent },
  error: { main: ERROR },
  background: { default: c.base, paper: c.surface },
  text: { primary: c.text, secondary: c.textSecondary, disabled: c.textSubdued },
  divider,
});

// Snowify only ships a light variant of its default theme. The other themes reuse its neutral
// light surfaces, with their accent darkened until it has 4.5:1 contrast on the surface.
const LIGHT = {
  base: "#f0f0f0",
  surface: "#fafafa",
  text: "#1a1a1a",
  textSecondary: "#555555",
  textSubdued: "#999999",
};

// Snowify draws its surfaces translucent over the base color. They are flattened here so the
// result is opaque, which is what a MUI palette expects.
const defs = {
  violet: {
    dark: { accent: "#aa55e6", base: "#030303", surface: "#080808", text: "#ffffff", textSecondary: "#b3b3b3", textSubdued: "#6a6a6a" },
    light: { ...LIGHT, accent: "#8b3dc7" },
  },
  ocean: {
    dark: { accent: "#5b8dee", base: "#0b0e14", surface: "#0f131d", text: "#eaf0ff", textSecondary: "#a0aec0", textSubdued: "#5a6680" },
    light: { ...LIGHT, accent: "#2c6ce9" },
  },
  forest: {
    dark: { accent: "#48bb78", base: "#0a120e", surface: "#0e1612", text: "#e6f5ec", textSecondary: "#8fbfa5", textSubdued: "#4e7a62" },
    light: { ...LIGHT, accent: "#308252" },
  },
  sunset: {
    dark: { accent: "#ed6450", base: "#12090e", surface: "#160d12", text: "#fde8e4", textSecondary: "#c09a94", textSubdued: "#7a5a54" },
    light: { ...LIGHT, accent: "#db3017" },
  },
  rose: {
    dark: { accent: "#db7093", base: "#100a10", surface: "#140e14", text: "#f5e6ee", textSecondary: "#b898aa", textSubdued: "#74526a" },
    light: { ...LIGHT, accent: "#ce3b6b" },
  },
  midnight: {
    dark: { accent: "#7b7bda", base: "#08080f", surface: "#0c0c15", text: "#e4e4f5", textSecondary: "#9898b8", textSubdued: "#55557a" },
    light: { ...LIGHT, accent: "#6767d4" },
  },
} satisfies Record<string, Record<Mode, Colors>>;

export type ThemeName = keyof typeof defs;
export const themeNames = Object.keys(defs) as ThemeName[];
export const DEFAULT_THEME: ThemeName = "violet";

const make = (mode: Mode, colors: Colors) =>
  createTheme({
    shape: { borderRadius: 10 },
    typography: { fontFamily: "Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif" },
    palette: {
      ...palette(colors, mode === "dark" ? "rgba(255, 255, 255, 0.07)" : "rgba(0, 0, 0, 0.08)"),
      mode,
    },
  });

export const themes = Object.fromEntries(
  themeNames.map((n) => [n, { light: make("light", defs[n].light), dark: make("dark", defs[n].dark) }]),
) as Record<ThemeName, Record<Mode, Theme>>;

const capitalize = (s: string) => s.replace(/^./, (c) => c.toUpperCase());
export const themeLabel = (n: ThemeName) => capitalize(n);
export const modeLabel = (m: ModePref) => capitalize(m);
