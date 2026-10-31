import { createContext, useContext, useLayoutEffect, type ReactNode } from "react";
import { CssBaseline, ThemeProvider, useMediaQuery } from "@mui/material";
import { useSetting } from "@/lib/settings";
import { DEFAULT_MODE, DEFAULT_THEME, modePrefs, themes, type ModePref, type ThemeName } from "./themes";

type Ctx = {
  name: ThemeName;
  setName: (n: ThemeName) => void;
  mode: ModePref;
  setMode: (m: ModePref) => void;
};
const ThemeCtx = createContext<Ctx | null>(null);

const isThemeName = (v: unknown): v is ThemeName => typeof v === "string" && v in themes;
const isModePref = (v: unknown): v is ModePref => modePrefs.includes(v as ModePref);

/** Read by the inline script in `index.html`. */
const BOOT_BG_KEY = "yamp-boot-bg";

export function AppThemeProvider({ children }: { children: ReactNode }) {
  const [name, setName] = useSetting<ThemeName>("theme", DEFAULT_THEME, isThemeName);
  const [mode, setMode] = useSetting<ModePref>("mode", DEFAULT_MODE, isModePref);

  const systemDark = useMediaQuery("(prefers-color-scheme: dark)");
  const resolved = mode === "system" ? (systemDark ? "dark" : "light") : mode;

  // index.html paints the page before any script runs. Keeping that color in step with the theme
  // and remembering it means the next launch starts on the right one instead of flashing.
  const bg = themes[name][resolved].palette.background.default;
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
      <ThemeProvider theme={themes[name][resolved]}>
        <CssBaseline />
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
