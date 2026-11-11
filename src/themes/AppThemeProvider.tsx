import { createContext, useContext, useLayoutEffect, useMemo, type ReactNode } from "react";
import { CssBaseline, GlobalStyles, ThemeProvider, useMediaQuery } from "@mui/material";
import { createTheme } from "@mui/material/styles";
import { isBoolean, useSetting } from "@/lib/settings";
import { IS_LINUX, IS_WINDOWS } from "@/util/platform";
import { useResolvedTheme } from "./custom";
import { DEFAULT_MODE, DEFAULT_THEME, modePrefs, type ModePref } from "./themes";

type Ctx = {
  /** A built-in theme name or the id of a custom one, see `custom.ts`. */
  name: string;
  setName: (n: string) => void;
  mode: ModePref;
  setMode: (m: ModePref) => void;
};
const ThemeCtx = createContext<Ctx | null>(null);

const isThemeName = (v: unknown): v is string => typeof v === "string";
const isModePref = (v: unknown): v is ModePref => modePrefs.includes(v as ModePref);

/**
 * With tooltips off every tooltip in the app stays closed, which is done once here and not per
 * usage. The listeners are what open a tooltip, so removing them keeps the wrapped control as is.
 */
const noTooltips = {
  MuiTooltip: { defaultProps: { disableHoverListener: true, disableFocusListener: true, disableTouchListener: true } },
};

/**
 * The webviews on Windows and Linux draw classic scrollbars at the sides of every scrolling
 * element. macOS and the mobile platforms use overlay bars that take no space. The layout is sized
 * to the window and scrolls with wheel and touch, so the classic bars only take space and add a
 * horizontal one when something is a pixel too wide.
 */
const noScrollbars = {
  "*": { scrollbarWidth: "none" },
  "*::-webkit-scrollbar": { display: "none" },
  "html, body": { overflow: "hidden" },
} as const;

/** Read by the inline script in `index.html`. */
const BOOT_BG_KEY = "yamp-boot-bg";

export function AppThemeProvider({ children }: { children: ReactNode }) {
  const [name, setName] = useSetting<string>("theme", DEFAULT_THEME, isThemeName);
  const [mode, setMode] = useSetting<ModePref>("mode", DEFAULT_MODE, isModePref);
  const [tooltips] = useSetting("tooltips", true, isBoolean);

  const systemDark = useMediaQuery("(prefers-color-scheme: dark)");
  const resolved = mode === "system" ? (systemDark ? "dark" : "light") : mode;

  // index.html paints the page before any script runs. Keeping that color in step with the theme
  // and remembering it means the next launch starts on the right one instead of flashing.
  const base = useResolvedTheme(name, resolved);
  const theme = useMemo(() => (tooltips ? base : createTheme(base, { components: noTooltips })), [base, tooltips]);
  const bg = base.palette.background.default;
  useLayoutEffect(() => {
    document.documentElement.style.background = bg;
    try {
      localStorage.setItem(BOOT_BG_KEY, bg);
    } catch {
      // storage can be unavailable, the next launch then starts on the default dark color
    }
  }, [bg]);

  return (
    <ThemeCtx.Provider value={{ name, setName, mode, setMode }}>
      <ThemeProvider theme={theme}>
        <CssBaseline />
        {(IS_WINDOWS || IS_LINUX) && <GlobalStyles styles={noScrollbars} />}
        {children}
      </ThemeProvider>
    </ThemeCtx.Provider>
  );
}

export function useAppTheme() {
  const ctx = useContext(ThemeCtx);
  if (!ctx) throw new Error("useAppTheme must be used inside AppThemeProvider");
  return ctx;
}
