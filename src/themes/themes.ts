import { alpha, createTheme, type PaletteOptions, type Theme } from "@mui/material/styles";

export type Mode = "light" | "dark";

/** What the user picks. `system` follows the OS and resolves to one of `Mode`. */
export const modePrefs = ["system", "dark", "light"] as const;
export type ModePref = (typeof modePrefs)[number];
export const DEFAULT_MODE: ModePref = "dark";

/** What a theme is made of, for a built-in one and for one the user defines alike. */
export type Colors = {
  accent: string;
  base: string;
  surface: string;
  text: string;
  textSecondary: string;
  textSubdued: string;
};

// One red for every theme, including the light one.
const ERROR = "#f06070";

const palette = (c: Colors, divider: string): PaletteOptions => ({
  primary: { main: c.accent },
  error: { main: ERROR },
  background: { default: c.base, paper: c.surface },
  text: { primary: c.text, secondary: c.textSecondary, disabled: c.textSubdued },
  divider,
});

// Every theme shares the same neutral light surfaces. Its accent is darkened until it has 4.5:1
// contrast on the surface.
const LIGHT = {
  base: "#f0f0f0",
  surface: "#fafafa",
  text: "#1a1a1a",
  textSecondary: "#555555",
  textSubdued: "#999999",
};

// The surfaces are meant to sit translucent over the base color. They are flattened here so the
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
  ember: {
    dark: { accent: "#f28c45", base: "#100b08", surface: "#17100c", text: "#fff0e5", textSecondary: "#c9a995", textSubdued: "#80695b" },
    light: { ...LIGHT, accent: "#bd571c" },
  },
  arctic: {
    dark: { accent: "#55c9e8", base: "#080f14", surface: "#0d171e", text: "#e6faff", textSecondary: "#9bbac6", textSubdued: "#526f7c" },
    light: { ...LIGHT, accent: "#167c9b" },
  },
  amethyst: {
    dark: { accent: "#b18aff", base: "#0e0a16", surface: "#15101f", text: "#f0eaff", textSecondary: "#b2a4cc", textSubdued: "#706080" },
    light: { ...LIGHT, accent: "#7545bd" },
  },
  sakura: {
    dark: { accent: "#f08caf", base: "#140a10", surface: "#1c1017", text: "#ffeaf1", textSecondary: "#c9a0b0", textSubdued: "#80596b" },
    light: { ...LIGHT, accent: "#bd456f" },
  },
  emerald: {
    dark: { accent: "#34d399", base: "#07120e", surface: "#0c1913", text: "#e2fff1", textSecondary: "#91bca5", textSubdued: "#4d7861" },
    light: { ...LIGHT, accent: "#16764e" },
  },
  cobalt: {
    dark: { accent: "#528bff", base: "#080d18", surface: "#0e1422", text: "#e8efff", textSecondary: "#a0afd0", textSubdued: "#596b91" },
    light: { ...LIGHT, accent: "#285ac0" },
  },
  gold: {
    dark: { accent: "#e8bd55", base: "#110e08", surface: "#19150c", text: "#fff8e5", textSecondary: "#c5b58c", textSubdued: "#796b4d" },
    light: { ...LIGHT, accent: "#936b13" },
  },
  mint: {
    dark: { accent: "#5eead4", base: "#071210", surface: "#0c1916", text: "#e5fffa", textSecondary: "#99c9c0", textSubdued: "#557e76" },
    light: { ...LIGHT, accent: "#167e70" },
  },
  crimson: {
    dark: { accent: "#ef5350", base: "#130809", surface: "#1b0e10", text: "#ffe9e9", textSecondary: "#c49b9e", textSubdued: "#80585c" },
    light: { ...LIGHT, accent: "#bd3030" },
  },
  lavender: {
    dark: { accent: "#aaa4ff", base: "#0d0c16", surface: "#141321", text: "#f0efff", textSecondary: "#aaa8c9", textSubdued: "#696783" },
    light: { ...LIGHT, accent: "#6258c2" },
  },
  copper: {
    dark: { accent: "#d98b65", base: "#120c09", surface: "#1a120e", text: "#fff0e7", textSecondary: "#c1a18e", textSubdued: "#806657" },
    light: { ...LIGHT, accent: "#a4512e" },
  },
  monochrome: {
    dark: { accent: "#b0b0b0", base: "#090909", surface: "#121212", text: "#f5f5f5", textSecondary: "#aaaaaa", textSubdued: "#666666" },
    light: { ...LIGHT, accent: "#555555" },
  },
} satisfies Record<string, Record<Mode, Colors>>;

export type ThemeName = keyof typeof defs;
export const themeNames = Object.keys(defs) as ThemeName[];
export const DEFAULT_THEME: ThemeName = "violet";

export const buildTheme = (mode: Mode, colors: Colors) => {
  const base = createTheme({
    shape: { borderRadius: 10 },
    typography: { fontFamily: "Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif" },
    palette: {
      ...palette(colors, mode === "dark" ? "rgba(255, 255, 255, 0.07)" : "rgba(0, 0, 0, 0.08)"),
      mode,
    },
  });
  const { primary } = base.palette;
  // Flat fills read as dead on a near black page. The depth comes from the accent itself, as an
  // even tint over the page and a short gradient on what is filled with it, kept low so the
  // themes stay calm.
  return createTheme(base, {
    components: {
      MuiCssBaseline: {
        styleOverrides: {
          // A tint that is the same across the whole window and only deepens toward a corner. A
          // spot of light in one place pulls the eye to it and leaves the rest of the page dull.
          body: {
            backgroundImage: `linear-gradient(160deg, ${alpha(primary.main, mode === "dark" ? 0.1 : 0.07)}, ${alpha(primary.main, mode === "dark" ? 0.03 : 0.02)} 60%, ${alpha(primary.main, mode === "dark" ? 0.07 : 0.05)})`,
            backgroundAttachment: "fixed",
          },
        },
      },
      MuiButton: {
        styleOverrides: {
          containedPrimary: {
            backgroundImage: `linear-gradient(135deg, ${primary.light}, ${primary.main})`,
            "&:hover": { backgroundImage: `linear-gradient(135deg, ${primary.main}, ${primary.dark})` },
          },
        },
      },
    },
  });
};

export const themes = Object.fromEntries(
  themeNames.map((n) => [n, { light: buildTheme("light", defs[n].light), dark: buildTheme("dark", defs[n].dark) }]),
) as Record<ThemeName, Record<Mode, Theme>>;

const capitalize = (s: string) => s.replace(/^./, (c) => c.toUpperCase());
/** The colors of a built-in theme, which is where a custom one can start from. */
export const builtinColors = (name: ThemeName, mode: Mode): Colors => defs[name][mode];

export const themeLabel = (n: ThemeName) => capitalize(n);
export const modeLabel = (m: ModePref) => capitalize(m);
